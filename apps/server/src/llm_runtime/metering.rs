//! 每次 turn 独立计量，observer 透传快照，future 被丢弃时将调用收束为 cancelled。
use super::{RuntimeState, RuntimeStore, now_ms, settings};
use crate::state::AppState;
use meowlive_application::ports::{
    llm::{DecisionRequest, LanguageModel, LlmError},
    llm_runtime::{ModelEvent, ModelObserver, ModelOptions, ModelTurn, TokenUsage},
};
use meowlive_protocol::llm_runtime::{LlmTokenUsage, LlmUsageRecord};
use std::{sync::Arc, time::Instant};

pub async fn measured_turn(
    state: &AppState,
    model: &dyn LanguageModel,
    request: DecisionRequest,
    mut options: ModelOptions,
) -> Result<ModelTurn, LlmError> {
    let started = Instant::now();
    let id = uuid::Uuid::new_v4().to_string();
    {
        let mut inner = state.llm_runtime.inner.lock().unwrap();
        if inner.pending.len() >= 128 {
            return Err(LlmError::new("模型调用并发已达到计量上限", true));
        }
        let config = &state.config.llm;
        inner.pending.insert(
            id.clone(),
            LlmUsageRecord {
                id: id.clone(),
                started_at_ms: now_ms(),
                provider: config.provider.clone(),
                api_format: config.api_format.clone(),
                base_url: if settings::validate_url(&config.base_url).is_ok() {
                    config.base_url.clone()
                } else {
                    String::new()
                },
                model: config.model.clone(),
                operation: "agent".into(),
                status: "running".into(),
                latency_ms: 0,
                first_token_ms: None,
                usage: LlmTokenUsage::default(),
                estimated_cost_microusd: None,
            },
        );
        persist_pending(&mut inner);
    }
    let mut guard = CallGuard {
        store: state.llm_runtime.clone(),
        id: id.clone(),
        started,
        finished: false,
    };
    options.observer = Some(Arc::new(Observer {
        store: state.llm_runtime.clone(),
        id,
        started,
        upstream: options.observer.take(),
    }));
    let result = model.turn(request, options).await;
    guard.finish(
        if result.is_ok() {
            "completed"
        } else {
            "failed"
        },
        result.as_ref().ok().map(|turn| &turn.usage),
    );
    result
}
struct Observer {
    store: Arc<RuntimeStore>,
    id: String,
    started: Instant,
    upstream: Option<Arc<dyn ModelObserver>>,
}
impl ModelObserver for Observer {
    fn on_event(&self, event: ModelEvent) {
        if !matches!(event, ModelEvent::OutputProgress { .. }) {
            let mut inner = self.store.inner.lock().unwrap();
            if let Some(record) = inner.pending.get_mut(&self.id) {
                record.latency_ms = elapsed(self.started);
                match &event {
                    ModelEvent::FirstToken => {
                        record.first_token_ms.get_or_insert(record.latency_ms);
                    }
                    ModelEvent::Usage(usage) => merge(&mut record.usage, usage),
                    ModelEvent::OutputProgress { .. } => {}
                }
                persist_pending(&mut inner);
            }
        }
        if let Some(upstream) = &self.upstream {
            upstream.on_event(event);
        }
    }
}
struct CallGuard {
    store: Arc<RuntimeStore>,
    id: String,
    started: Instant,
    finished: bool,
}
impl CallGuard {
    fn finish(&mut self, status: &str, usage: Option<&TokenUsage>) {
        if self.finished {
            return;
        }
        self.finished = true;
        let mut inner = self.store.inner.lock().unwrap();
        let Some(mut record) = inner.pending.remove(&self.id) else {
            return;
        };
        record.status = status.into();
        record.latency_ms = elapsed(self.started);
        if let Some(usage) = usage {
            merge(&mut record.usage, usage);
        }
        let persisted = if inner.healthy {
            match &mut inner.disk {
                Some(disk) => disk.append(&record).is_ok(),
                None => false,
            }
        } else {
            false
        };
        if !persisted {
            if inner.disk.is_some() {
                inner.healthy = false;
            }
            inner.memory_records.push(record);
            if inner.disk.is_some() && inner.memory_records.len() > 200 {
                inner.memory_records.remove(0);
            }
        }
        persist_pending(&mut inner);
    }
}
impl Drop for CallGuard {
    fn drop(&mut self) {
        self.finish("cancelled", None);
    }
}
fn persist_pending(inner: &mut RuntimeState) {
    if inner.healthy
        && inner
            .disk
            .as_ref()
            .is_some_and(|disk| disk.save_pending(&inner.pending).is_err())
    {
        inner.healthy = false;
    }
}
fn elapsed(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}
fn merge(target: &mut LlmTokenUsage, snapshot: &TokenUsage) {
    // Snapshot counters overwrite, rather than add. An absent field never erases a
    // counter already reported by an earlier event or preceding the final error.
    if snapshot.input_tokens.is_some() {
        target.input_tokens = snapshot.input_tokens;
    }
    if snapshot.output_tokens.is_some() {
        target.output_tokens = snapshot.output_tokens;
    }
    if snapshot.cache_read_tokens.is_some() {
        target.cache_read_tokens = snapshot.cache_read_tokens;
    }
    if snapshot.cache_write_tokens.is_some() {
        target.cache_write_tokens = snapshot.cache_write_tokens;
    }
    if snapshot.reasoning_tokens.is_some() {
        target.reasoning_tokens = snapshot.reasoning_tokens;
    }
}
