mod agent_support;
mod support;
use agent_support::*;
use meowlive_application::ports::llm::{DecisionFuture, DecisionRequest, LanguageModel};
use meowlive_protocol::agent::{AgentEventStatus, EventBatchRequest};
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;

struct BlockedModel {
    entered: Arc<Notify>,
    dropped: Arc<Notify>,
}
struct Guard(Arc<Notify>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}
impl LanguageModel for BlockedModel {
    fn decide(&self, _: DecisionRequest) -> DecisionFuture<'_> {
        let guard = Guard(self.dropped.clone());
        self.entered.notify_one();
        Box::pin(async move {
            let _guard = guard;
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn pause_stop_settings_and_disconnect_drop_inflight_model_without_queued_speech() {
    for action in ["pause", "stop", "settings", "disconnect"] {
        let entered = Arc::new(Notify::new());
        let dropped = Arc::new(Notify::new());
        let harness = Harness::new(Arc::new(BlockedModel {
            entered: entered.clone(),
            dropped: dropped.clone(),
        }))
        .await;
        let (control, audio, _) = support::pair(&harness.base).await;
        support::await_connected(&harness.state, true).await;
        harness
            .state
            .submit_events(EventBatchRequest {
                events: vec![event("event-1")],
            })
            .await
            .unwrap();
        harness.state.resume_agent().await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), entered.notified())
            .await
            .unwrap();
        match action {
            "pause" => {
                harness.state.pause_agent().await;
            }
            "stop" => {
                harness.state.stop().await;
            }
            "settings" => {
                let mut settings = harness.state.agent_snapshot().await.settings;
                settings.persona = "另一位主播".into();
                harness.state.configure_agent(settings).await.unwrap();
            }
            "disconnect" => {
                harness.state.shutdown().await;
            }
            _ => unreachable!(),
        }
        tokio::time::timeout(Duration::from_millis(300), dropped.notified())
            .await
            .expect(action);
        assert!(harness.state.snapshot().await.speeches.is_empty());
        let snapshot = harness.state.agent_snapshot().await;
        assert!(snapshot.paused);
        assert_eq!(snapshot.events[0].status, AgentEventStatus::Cancelled);
        drop(control);
        drop(audio);
    }
}
