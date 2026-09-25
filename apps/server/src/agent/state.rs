use super::mapping;
use crate::{
    state::{AppState, Inner},
    transport::error::ApiError,
};
use axum::http::StatusCode;
use meowlive_application::agent::{AgentSettings, SubmitOutcome};
use meowlive_domain::speech::{SpeechStatus, SpeechTask};
use meowlive_protocol::agent::{
    AgentSettings as SettingsDto, AgentSnapshot, EventBatchRequest, EventBatchResult,
};
use tokio_util::sync::CancellationToken;

impl AppState {
    pub(crate) fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }
    pub(crate) fn sync_agent(&self, inner: &mut Inner) {
        if let Some(task) = self.agent_speech(inner) {
            self.agent_observability.sync_speech(&task);
            inner.agent.sync_speech(&task, self.now_ms());
        }
    }
    pub(crate) fn agent_speech(&self, inner: &mut Inner) -> Option<SpeechTask> {
        inner
            .agent
            .view(self.now_ms())
            .current_speech_id
            .and_then(|id| inner.queue.get(&id).cloned())
    }
    pub(crate) fn end_agent_speech(
        &self,
        inner: &mut Inner,
        task: Option<SpeechTask>,
        disconnected: bool,
    ) {
        if let Some(mut task) = task {
            if let Some(final_task) = inner.queue.get(&task.id) {
                task = final_task.clone();
            } else {
                task.status = if disconnected
                    && matches!(task.status, SpeechStatus::Ready | SpeechStatus::Playing)
                {
                    SpeechStatus::Unknown
                } else {
                    SpeechStatus::Cancelled
                };
                task.error = None;
            }
            self.agent_observability.sync_speech(&task);
            inner.agent.sync_speech(&task, self.now_ms());
        }
    }
    pub async fn agent_snapshot(&self) -> AgentSnapshot {
        let mut inner = self.inner.lock().await;
        self.sync_agent(&mut inner);
        let connected = inner.queue.is_connected();
        mapping::snapshot(
            inner.agent.view(self.now_ms()),
            self.model.is_some(),
            connected,
        )
    }
    pub async fn configure_agent(&self, settings: SettingsDto) -> Result<AgentSnapshot, ApiError> {
        let settings = AgentSettings {
            persona: settings.persona,
            system_prompt: settings.system_prompt,
            topic: settings.topic,
            proactive_enabled: settings.proactive_enabled,
            cooldown_ms: u64::from(settings.cooldown_ms),
            interaction: crate::agent::mapping::interaction(settings.interaction),
        };
        settings.validate().map_err(invalid)?;
        let mut inner = self.inner.lock().await;
        self.agent_settings.save(&settings).map_err(|message| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "agent_settings_save_failed",
                message,
            )
        })?;
        inner
            .agent
            .configure(settings, self.now_ms())
            .map_err(invalid)?;
        inner.agent_cancel.cancel();
        inner.agent_cancel = CancellationToken::new();
        self.agent_wake.notify_one();
        drop(inner);
        Ok(self.agent_snapshot().await)
    }
    pub async fn pause_agent(&self) -> AgentSnapshot {
        let mut inner = self.inner.lock().await;
        inner.agent.set_paused(true, self.now_ms());
        inner.agent_cancel.cancel();
        inner.agent_cancel = CancellationToken::new();
        self.agent_wake.notify_one();
        drop(inner);
        self.agent_snapshot().await
    }
    pub async fn resume_agent(&self) -> Result<AgentSnapshot, ApiError> {
        let mut inner = self.inner.lock().await;
        self.require_gpu_idle()?;
        if self.model.is_none() {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "llm_unconfigured",
                "请在 LLM 接入页配置接口、模型和密钥，保存后重启主服务",
            ));
        }
        if !inner.queue.is_connected() {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "bridge_disconnected",
                "请先连接桌面执行端",
            ));
        }
        inner.agent.set_paused(false, self.now_ms());
        self.agent_wake.notify_one();
        drop(inner);
        Ok(self.agent_snapshot().await)
    }
    pub async fn submit_events(
        &self,
        batch: EventBatchRequest,
    ) -> Result<EventBatchResult, ApiError> {
        if batch.events.is_empty() || batch.events.len() > 100 {
            return Err(invalid("每批事件须为 1–100 条".into()));
        }
        let now = self.now_ms();
        let events: Vec<_> = batch
            .events
            .into_iter()
            .map(|e| {
                mapping::event(
                    e,
                    if self.viewer_store.is_some() {
                        crate::viewers::utc_ms()
                    } else {
                        now
                    },
                )
            })
            .collect();
        for event in &events {
            event.validate().map_err(invalid)?;
        }
        if self.viewer_store.is_some() {
            return self.submit_persisted_events(events).await;
        }
        if self.config.viewers.enabled {
            return Err(crate::viewers::unavailable());
        }
        let mut inner = self.inner.lock().await;
        // Bounded copy makes batch admission all-or-nothing even on capacity errors.
        let mut staged = inner.agent.clone();
        let mut result = EventBatchResult {
            accepted: 0,
            duplicates: 0,
            persisted: None,
            unscheduled: None,
        };
        for event in events {
            let age = if matches!(
                event.kind,
                meowlive_domain::event::EventKind::SuperChat { .. }
            ) {
                crate::viewers::event_age_ms(&event, crate::viewers::utc_ms())
            } else {
                0
            };
            match staged.submit_with_age(event, now, age).map_err(|message| {
                if message.contains("capacity") || message.contains("full") {
                    ApiError::new(
                        StatusCode::TOO_MANY_REQUESTS,
                        "event_queue_full",
                        "待回应事件已满，请等待处理或立即停止以清空",
                    )
                } else {
                    invalid(message)
                }
            })? {
                SubmitOutcome::Accepted => result.accepted += 1,
                SubmitOutcome::Duplicate => result.duplicates += 1,
            }
        }
        inner.agent = staged;
        self.agent_wake.notify_one();
        Ok(result)
    }
}
fn invalid(message: String) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "invalid_agent_request", message)
}
