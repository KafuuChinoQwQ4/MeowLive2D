mod agent_support;
mod support;
use agent_support::*;
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use meowlive_protocol::agent::{AgentEventStatus, EventBatchRequest};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[tokio::test]
async fn simulated_event_reaches_completed_only_after_desktop_playout() {
    let calls = Arc::new(AtomicUsize::new(0));
    let harness = Harness::new(Arc::new(ReplyModel {
        calls: calls.clone(),
    }))
    .await;
    let config = ClientConfig::from_toml(&format!(
        "server_url='{}'",
        harness.base.replace("ws://", "http://")
    ))
    .unwrap();
    let desktop = tokio::spawn(async move {
        run_once(&config, SimulatedBackend::new(config.max_buffer_samples)).await
    });
    support::await_connected(&harness.state, true).await;
    harness
        .state
        .submit_events(EventBatchRequest {
            events: vec![event("event-1")],
        })
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    harness.state.resume_agent().await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot = harness.state.agent_snapshot().await;
            if snapshot.events[0].status == AgentEventStatus::Completed {
                assert!(snapshot.events[0].speech_id.is_some());
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let result = harness
        .state
        .submit_events(EventBatchRequest {
            events: vec![event("event-1")],
        })
        .await
        .unwrap();
    assert_eq!(result.duplicates, 1);
    harness.state.shutdown().await;
    assert!(desktop.await.unwrap().is_err());
}

#[tokio::test]
async fn event_is_queued_before_receipt_and_disconnect_marks_it_unknown_without_replay() {
    let calls = Arc::new(AtomicUsize::new(0));
    let harness = Harness::new(Arc::new(ReplyModel {
        calls: calls.clone(),
    }))
    .await;
    let (mut control, mut audio, _) = support::pair(&harness.base).await;
    support::await_connected(&harness.state, true).await;
    harness
        .state
        .submit_events(EventBatchRequest {
            events: vec![event("event-1")],
        })
        .await
        .unwrap();
    harness.state.resume_agent().await.unwrap();
    let speak = support::next_json(&mut control).await;
    assert_eq!(speak["type"], "speak");
    support::next_data(&mut audio).await;
    let snapshot = harness.state.agent_snapshot().await;
    assert_eq!(snapshot.events[0].status, AgentEventStatus::Ready);
    drop(control);
    drop(audio);
    support::await_connected(&harness.state, false).await;
    let snapshot = harness.state.agent_snapshot().await;
    assert!(snapshot.paused);
    assert_eq!(snapshot.events[0].status, AgentEventStatus::Unknown);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
