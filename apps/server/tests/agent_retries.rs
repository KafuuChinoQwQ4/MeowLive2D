mod agent_support;
mod support;
use agent_support::*;
use meowlive_application::ports::llm::{
    AgentDecision, DecisionFuture, DecisionRequest, LanguageModel, LlmError,
};
use meowlive_protocol::{
    agent::{AgentEventStatus, EventBatchRequest},
    agent_observability::AgentTraceStatus,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

struct RetryModel {
    calls: Arc<AtomicUsize>,
    retryable: bool,
    always_fail: bool,
}
impl LanguageModel for RetryModel {
    fn decide(&self, request: DecisionRequest) -> DecisionFuture<'_> {
        let attempt = self.calls.fetch_add(1, Ordering::SeqCst);
        let retryable = self.retryable;
        let fail = self.always_fail || attempt == 0;
        Box::pin(async move {
            if fail {
                Err(LlmError::new("模型暂不可用", retryable))
            } else {
                Ok(AgentDecision {
                    reply_to: request.events.into_iter().map(|e| e.id).collect(),
                    text: Some("恢复回复".into()),
                    topic: None,
                })
            }
        })
    }
}

#[tokio::test]
async fn only_transient_errors_retry_and_total_attempts_are_bounded() {
    for (retryable, always_fail, expected_calls, expected_status) in [
        (true, false, 2, AgentEventStatus::Ready),
        (false, true, 1, AgentEventStatus::Failed),
        (true, true, 2, AgentEventStatus::Failed),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let harness = Harness::new(Arc::new(RetryModel {
            calls: calls.clone(),
            retryable,
            always_fail,
        }))
        .await;
        let (control, audio, _) = support::pair(&harness.base).await;
        support::await_connected(&harness.state, true).await;
        harness
            .state
            .submit_events(EventBatchRequest {
                events: vec![event("e1")],
            })
            .await
            .unwrap();
        harness.state.resume_agent().await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if harness.state.agent_snapshot().await.events[0].status == expected_status {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), expected_calls);
        let summary = harness.state.agent_observability.list(1, None).traces[0].clone();
        let trace = harness.state.agent_observability.get(&summary.id).unwrap();
        assert_eq!(trace.turns.len(), expected_calls);
        assert_eq!(trace.turns[0].retry_attempt, 0);
        if expected_calls == 2 {
            assert_eq!(trace.turns[1].retry_attempt, 1);
        }
        assert_eq!(
            summary.status,
            if expected_status == AgentEventStatus::Failed {
                AgentTraceStatus::Failed
            } else {
                AgentTraceStatus::Running
            }
        );
        drop(control);
        drop(audio);
    }
}
