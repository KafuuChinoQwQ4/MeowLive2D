//! 观察链路在上下文取消、资料失效和语音阶段变化时的回归测试。
use super::*;
use meowlive_application::ports::{
    companionship::*,
    llm::DecisionFuture,
    memory::EmbeddingBatch,
    memory_store::*,
    speech::{SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
    viewers::ViewerStoreFuture,
};
use meowlive_domain::{
    event::{EventKind, LiveEvent},
    memory::MemoryCandidate,
    speech::SpeechReceiptStatus,
};
use std::sync::Arc;
use tokio::sync::Notify;

struct NoSpeech;
impl SpeechSynthesizer for NoSpeech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async { panic!("speech must not run in this test") })
    }
}

struct HeldModel(Arc<Notify>);
impl LanguageModel for HeldModel {
    fn decide(&self, _: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async move {
            self.0.notify_one();
            std::future::pending().await
        })
    }
}

struct HeldMemory {
    entered: Arc<Notify>,
    hold_revision: bool,
}
#[allow(unused_variables)]
impl MemoryStore for HeldMemory {
    fn retry_failed<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryMaintenanceRequest,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn rebuild_vectors<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryMaintenanceRequest,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn resolve_viewers<'a>(
        &'a self,
        scope: &'a str,
        events: &'a [meowlive_domain::event::LiveEvent],
    ) -> ViewerStoreFuture<'a, Vec<String>> {
        Box::pin(async move {
            if self.hold_revision {
                return Ok(vec!["viewer".into()]);
            }
            self.entered.notify_one();
            std::future::pending().await
        })
    }
    fn claim_job<'a>(
        &'a self,
        scope: &'a str,
        now: i64,
    ) -> ViewerStoreFuture<'a, Option<ExtractionJob>> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn finish_job<'a>(
        &'a self,
        scope: &'a str,
        job: &'a ExtractionJob,
        candidates: &'a [MemoryCandidate],
        now: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn fail_job<'a>(
        &'a self,
        scope: &'a str,
        job: &'a ExtractionJob,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn list<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        limit: u32,
        now: i64,
    ) -> ViewerStoreFuture<'a, Vec<MemoryRecord>> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn context<'a>(
        &'a self,
        scope: &'a str,
        viewer_ids: &'a [String],
        query: Option<&'a QueryEmbedding>,
        now: i64,
    ) -> ViewerStoreFuture<'a, MemorySnapshot> {
        Box::pin(async {
            Ok(MemorySnapshot {
                revision: 1,
                records: vec![],
            })
        })
    }
    fn revision<'a>(&'a self, scope: &'a str) -> ViewerStoreFuture<'a, i64> {
        Box::pin(async move {
            self.entered.notify_one();
            std::future::pending().await
        })
    }
    fn admin_correct<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryAdminRequest,
        value: &'a str,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn admin_delete<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryAdminRequest,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn admin_freeze<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryAdminRequest,
        frozen: bool,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn claim_embedding<'a>(
        &'a self,
        scope: &'a str,
        model: &'a str,
        dimensions: usize,
        now: i64,
    ) -> ViewerStoreFuture<'a, Option<EmbeddingJob>> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn store_embedding<'a>(
        &'a self,
        scope: &'a str,
        job: &'a EmbeddingJob,
        batch: &'a EmbeddingBatch,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn status<'a>(&'a self, scope: &'a str) -> ViewerStoreFuture<'a, MemoryQueueStatus> {
        Box::pin(async { panic!("unused memory operation") })
    }
    fn maintenance<'a>(&'a self, scope: &'a str, now: i64) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused memory operation") })
    }
}

async fn ready_state() -> (AppState, Arc<Notify>) {
    let entered = Arc::new(Notify::new());
    let state = AppState::with_model(
        crate::config::AppConfig::default(),
        Arc::new(NoSpeech),
        Some(Arc::new(HeldModel(entered.clone()))),
    );
    {
        let mut inner = state.inner.lock().await;
        inner.queue.set_connected(true);
        inner
            .agent
            .submit(
                LiveEvent {
                    id: "event".into(),
                    source: "test".into(),
                    viewer: "观众".into(),
                    viewer_identity: None,
                    occurred_at_ms: 0,
                    gift_metadata: None,
                    kind: EventKind::Chat {
                        text: "你好".into(),
                    },
                },
                0,
            )
            .unwrap();
        inner.agent.set_paused(false, 0);
    }
    (state, entered)
}

async fn await_finished(
    state: &AppState,
) -> meowlive_protocol::agent_observability::AgentTraceSummary {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(trace) = state.agent_observability.list(10, None).traces.first() {
                if trace.status != AgentTraceStatus::Running {
                    return trace.clone();
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("cancellation must finish the trace without waiting for context/model I/O")
}

#[tokio::test]
async fn context_loading_can_be_cancelled_and_reports_knowledge_invalidation() {
    for operation in ["pause", "stop", "knowledge"] {
        let (mut state, model_entered) = ready_state().await;
        let context_entered = Arc::new(Notify::new());
        state.memory_store = Some(Arc::new(HeldMemory {
            entered: context_entered.clone(),
            hold_revision: false,
        }));
        let worker = tokio::spawn(run_agent(state.clone()));
        tokio::time::timeout(Duration::from_secs(2), context_entered.notified())
            .await
            .unwrap();
        match operation {
            "pause" => {
                state.pause_agent().await;
            }
            "stop" => {
                state.stop().await;
            }
            _ => state.invalidate_knowledge().await,
        }
        let trace = await_finished(&state).await;
        assert_eq!(trace.turn_count, 0);
        assert_eq!(
            trace.status,
            if operation == "knowledge" {
                AgentTraceStatus::Failed
            } else {
                AgentTraceStatus::Cancelled
            }
        );
        if operation == "knowledge" {
            assert!(trace.result.contains("观众资料"));
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(10), model_entered.notified())
                .await
                .is_err()
        );
        worker.abort();
        let _ = worker.await;
    }
}

#[tokio::test]
async fn knowledge_change_during_generation_is_a_failed_trace() {
    let (state, model_entered) = ready_state().await;
    let worker = tokio::spawn(run_agent(state.clone()));
    tokio::time::timeout(Duration::from_secs(2), model_entered.notified())
        .await
        .unwrap();
    state.invalidate_knowledge().await;
    let trace = await_finished(&state).await;
    assert_eq!(trace.status, AgentTraceStatus::Failed);
    assert!(trace.result.contains("观众资料"));
    assert_eq!(
        state.agent_observability.get(&trace.id).unwrap().turns[0].status,
        AgentTraceStatus::Cancelled
    );
    worker.abort();
    let _ = worker.await;
}

#[tokio::test]
async fn knowledge_change_finishes_queued_synthesizing_ready_and_playing_traces() {
    for phase in 0..4 {
        let (state, _) = ready_state().await;
        let trace_id = state.agent_observability.start_trace("live_events", vec![]);
        {
            let mut inner = state.inner.lock().await;
            let work = inner.agent.begin(0).unwrap();
            let prepared = inner
                .agent
                .resolve(
                    work.id,
                    AgentDecision {
                        reply_to: vec!["event".into()],
                        text: Some("你好呀".into()),
                        topic: None,
                    },
                    "speech".into(),
                    0,
                )
                .unwrap()
                .unwrap();
            inner
                .queue
                .enqueue_broadcast("speech", prepared.text, "default")
                .unwrap();
            state.agent_observability.bind_speech(&trace_id, "speech");
            let generation = inner.queue.generation();
            if phase >= 1 {
                inner.queue.next_for_synthesis().unwrap();
            }
            if phase >= 2 {
                assert!(inner.queue.mark_ready("speech", generation));
            }
            if phase >= 3 {
                assert!(inner.queue.mark_dispatched("speech", generation));
                assert!(inner.queue.apply_receipt(
                    "speech",
                    generation,
                    SpeechReceiptStatus::Started,
                    None
                ));
            }
        }
        state.invalidate_knowledge().await;
        let trace = state.agent_observability.get(&trace_id).unwrap();
        assert_eq!(
            trace.summary.status,
            AgentTraceStatus::Failed,
            "phase {phase}"
        );
        assert!(trace.summary.result.contains("观众资料"));
        assert!(state.agent_snapshot().await.current_speech_id.is_none());
    }
}

struct HeldRegistration(Arc<Notify>);
#[allow(unused_variables)]
impl CompanionshipStore for HeldRegistration {
    fn detail<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        limit: u32,
    ) -> ViewerStoreFuture<'a, Option<CompanionshipDetail>> {
        Box::pin(async { panic!("unused companionship operation") })
    }
    fn register_reply<'a>(
        &'a self,
        scope: &'a str,
        speech_id: &'a str,
        events: &'a [LiveEvent],
        now: u64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async move {
            self.0.notify_one();
            std::future::pending().await
        })
    }
    fn complete_reply<'a>(
        &'a self,
        scope: &'a str,
        speech_id: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async { panic!("unused companionship operation") })
    }
    fn adjust<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        request: &'a AffinityAdjustment,
        now: u64,
    ) -> ViewerStoreFuture<'a, String> {
        Box::pin(async { panic!("unused companionship operation") })
    }
    fn reverse<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        request: &'a AffinityReversal,
        now: u64,
    ) -> ViewerStoreFuture<'a, String> {
        Box::pin(async { panic!("unused companionship operation") })
    }
    fn confirm_gift<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        request: &'a GiftConfirmation,
        now: u64,
    ) -> ViewerStoreFuture<'a, String> {
        Box::pin(async { panic!("unused companionship operation") })
    }
}
struct ImmediateModel;
impl LanguageModel for ImmediateModel {
    fn decide(&self, request: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async move {
            Ok(AgentDecision {
                reply_to: request
                    .events
                    .iter()
                    .map(|event| event.id.clone())
                    .collect(),
                text: Some("你好呀".into()),
                topic: None,
            })
        })
    }
}

#[tokio::test]
async fn cancelling_post_model_revision_or_reply_registration_finishes_trace() {
    for stage in ["revision", "registration"] {
        let (mut state, _) = ready_state().await;
        state.model = Some(Arc::new(ImmediateModel));
        let entered = Arc::new(Notify::new());
        if stage == "revision" {
            state.memory_store = Some(Arc::new(HeldMemory {
                entered: entered.clone(),
                hold_revision: true,
            }));
        } else {
            state.companionship_store = Some(Arc::new(HeldRegistration(entered.clone())));
        }
        let worker = tokio::spawn(run_agent(state.clone()));
        tokio::time::timeout(Duration::from_secs(2), entered.notified())
            .await
            .unwrap();
        state.stop().await;
        let trace = await_finished(&state).await;
        assert_eq!(trace.status, AgentTraceStatus::Cancelled);
        assert!(trace.speech_id.is_none());
        assert!(state.agent_snapshot().await.current_speech_id.is_none());
        worker.abort();
        let _ = worker.await;
    }
}
