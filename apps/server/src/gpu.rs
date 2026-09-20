//! 在同一业务锁下准入 GPU 操作、音色切换与直播/播报，租约随任务结束释放。
use crate::{state::AppState, transport::error::ApiError};
use axum::http::StatusCode;
use meowlive_application::agent::AgentPhase;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) struct GpuLease {
    busy: Arc<AtomicBool>,
    wake: Arc<tokio::sync::Notify>,
}
impl Drop for GpuLease {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
        self.wake.notify_one();
    }
}
impl AppState {
    pub(crate) fn require_gpu_idle(&self) -> Result<(), ApiError> {
        if self.gpu_busy.load(Ordering::Acquire)
            || self.model_synthesizer.as_ref().is_some_and(|s| s.is_busy())
        {
            Err(ApiError::new(
                StatusCode::CONFLICT,
                "gpu_busy",
                "正在训练、试听或测量，请等待完成或取消训练",
            ))
        } else {
            Ok(())
        }
    }
    pub(crate) async fn acquire_gpu(&self) -> Result<GpuLease, ApiError> {
        self.acquire_model_operation(true).await
    }
    pub(crate) async fn acquire_model_selection(&self) -> Result<GpuLease, ApiError> {
        // Selection only updates metadata. Fence new speech and decisions during
        // the commit without changing the user's live session or pause choice.
        self.acquire_model_operation(false).await
    }
    async fn acquire_model_operation(&self, pause_session: bool) -> Result<GpuLease, ApiError> {
        let mut inner = self.inner.lock().await;
        self.require_gpu_idle()?;
        if self.stopping.is_cancelled()
            || (pause_session && inner.live.cancel.is_some())
            || inner.queue.tasks().any(|task| !task.status.is_terminal())
            || inner.agent.view(self.now_ms()).phase == AgentPhase::Deciding
            || self.resource_changing.load(Ordering::Acquire)
        {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "session_busy",
                if pause_session {
                    "请先结束直播、暂停自动回应并等待播报和推理完成"
                } else {
                    "正在播报、推理或修改资源，请等待完成后切换音色"
                },
            ));
        }
        if pause_session {
            inner.agent.set_paused(true, self.now_ms());
        }
        self.gpu_busy.store(true, Ordering::Release);
        Ok(GpuLease {
            busy: self.gpu_busy.clone(),
            wake: self.agent_wake.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meowlive_application::ports::speech::{
        SpeechSynthesizer, SynthesisFuture, SynthesisRequest,
    };
    struct NeverSpeech;
    impl SpeechSynthesizer for NeverSpeech {
        fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
            Box::pin(std::future::pending())
        }
    }
    #[tokio::test]
    async fn training_lease_fences_live_agent_and_manual_speech_and_releases_on_drop() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;
        let state = AppState::new(crate::config::AppConfig::default(), Arc::new(NeverSpeech));
        let lease = state.acquire_gpu().await.unwrap();
        assert_eq!(state.connect_live().await.unwrap_err().1, "gpu_busy");
        assert_eq!(state.resume_agent().await.unwrap_err().1, "gpu_busy");
        assert!(state.acquire_gpu().await.is_err());
        let response = crate::transport::http::router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/speech")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"hello","voice_id":"default"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(state.snapshot().await.speeches.is_empty());
        drop(lease);
        assert!(state.require_gpu_idle().is_ok());
        assert!(state.acquire_gpu().await.is_ok());
    }

    #[tokio::test]
    async fn model_selection_preserves_live_connection_and_user_pause_choice() {
        use tokio_util::sync::CancellationToken;
        for paused in [false, true] {
            let state = AppState::new(crate::config::AppConfig::default(), Arc::new(NeverSpeech));
            let live = CancellationToken::new();
            {
                let mut inner = state.inner.lock().await;
                inner.live.cancel = Some(live.clone());
                inner.agent.set_paused(paused, 0);
            }
            let lease = state.acquire_model_selection().await.unwrap();
            assert_eq!(state.agent_snapshot().await.paused, paused);
            assert!(!live.is_cancelled());
            assert!(state.acquire_model_selection().await.is_err());
            assert!(state.acquire_gpu().await.is_err());
            drop(lease);
            assert_eq!(state.agent_snapshot().await.paused, paused);
            assert!(state.inner.lock().await.live.cancel.is_some());
            assert!(state.require_gpu_idle().is_ok());
        }
    }

    #[tokio::test]
    async fn dropping_model_selection_does_not_override_a_concurrent_user_pause() {
        let state = AppState::new(crate::config::AppConfig::default(), Arc::new(NeverSpeech));
        state.inner.lock().await.agent.set_paused(false, 0);
        let lease = state.acquire_model_selection().await.unwrap();
        state.pause_agent().await;
        drop(lease);
        assert!(state.agent_snapshot().await.paused);
    }

    #[tokio::test]
    async fn model_selection_rejects_shutdown_pending_speech_decisions_and_resource_edits() {
        for reason in ["shutdown", "speech", "decision", "resource"] {
            let state = AppState::new(crate::config::AppConfig::default(), Arc::new(NeverSpeech));
            {
                let mut inner = state.inner.lock().await;
                inner.agent.set_paused(false, 0);
                match reason {
                    "shutdown" => state.stopping.cancel(),
                    "speech" => {
                        inner.queue.set_connected(true);
                        inner.queue.enqueue("pending", "hello", "default").unwrap();
                    }
                    "decision" => {
                        inner.agent.submit(event(), 0).unwrap();
                        assert!(inner.agent.begin(0).is_some());
                    }
                    "resource" => state.resource_changing.store(true, Ordering::Release),
                    _ => unreachable!(),
                }
            }
            assert!(state.acquire_model_selection().await.is_err(), "{reason}");
            assert!(!state.gpu_busy.load(Ordering::Acquire));
            assert!(!state.agent_snapshot().await.paused);
        }
    }

    fn event() -> meowlive_domain::event::LiveEvent {
        meowlive_domain::event::LiveEvent {
            id: "event".into(),
            source: "test".into(),
            viewer: "viewer".into(),
            viewer_identity: None,
            occurred_at_ms: 0,
            gift_metadata: None,
            kind: meowlive_domain::event::EventKind::Chat {
                text: "hello".into(),
            },
        }
    }

    #[tokio::test]
    async fn model_selection_fences_speech_and_agent_decisions_until_release() {
        use axum::{body::Body, http::Request};
        use meowlive_application::ports::llm::{DecisionFuture, DecisionRequest, LanguageModel};
        use tower::ServiceExt;
        struct StartedModel(Arc<tokio::sync::Notify>);
        impl LanguageModel for StartedModel {
            fn decide(&self, _: DecisionRequest) -> DecisionFuture<'_> {
                self.0.notify_one();
                Box::pin(std::future::pending())
            }
        }
        let started = Arc::new(tokio::sync::Notify::new());
        let state = AppState::with_model(
            crate::config::AppConfig::default(),
            Arc::new(NeverSpeech),
            Some(Arc::new(StartedModel(started.clone()))),
        );
        {
            let mut inner = state.inner.lock().await;
            inner.queue.set_connected(true);
            inner.agent.set_paused(false, 0);
            inner.agent.submit(event(), 0).unwrap();
        }
        let lease = state.acquire_model_selection().await.unwrap();
        let worker = tokio::spawn(crate::agent::run_agent(state.clone()));
        let response = crate::transport::http::router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/speech")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"hello","voice_id":"default"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(state.snapshot().await.speeches.is_empty());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(150), started.notified())
                .await
                .is_err()
        );
        drop(lease);
        tokio::time::timeout(std::time::Duration::from_secs(2), started.notified())
            .await
            .unwrap();
        assert!(!state.agent_snapshot().await.paused);
        worker.abort();
    }
}
