mod agent_support;
mod support;
use agent_support::*;
use futures_util::SinkExt;
use meowlive_protocol::agent::{AgentEventStatus, EventBatchRequest};
use meowlive_server::{config::AppConfig, transport::http::router};
use serde_json::json;
use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

#[tokio::test]
async fn stopping_newer_manual_speech_does_not_erase_a_completed_agent_reply() {
    let mut config = AppConfig::default();
    config.server.history_limit = 1;
    let harness = Harness::configured(
        config,
        Arc::new(ReplyModel {
            calls: Arc::new(AtomicUsize::new(0)),
        }),
    )
    .await;
    let (mut control, mut audio, _) = support::pair(&harness.base).await;
    support::await_connected(&harness.state, true).await;
    harness
        .state
        .submit_events(EventBatchRequest {
            events: vec![event("e1")],
        })
        .await
        .unwrap();
    harness.state.resume_agent().await.unwrap();
    let speak = support::next_json(&mut control).await;
    support::next_data(&mut audio).await;
    // Freeze background observation after it has enqueued its one utterance.
    harness.tasks[2].abort();
    for status in ["started", "completed"] {
        control.send(tokio_tungstenite::tungstenite::Message::text(json!({"type":"receipt","receipt":{
            "utterance_id":speak["utterance_id"],"generation":speak["generation"],"status":status,"error":null
        }}).to_string())).await.unwrap();
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while harness.state.snapshot().await.speeches[0].status
            != meowlive_protocol::control::SpeechStatus::Completed
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    support::request(
        router(harness.state.clone()),
        "POST",
        "/api/speech",
        json!({"text":"下一条人工语音","voice_id":"default"}),
    )
    .await;
    harness.state.stop().await;
    let snapshot = harness.state.agent_snapshot().await;
    assert_eq!(snapshot.events[0].status, AgentEventStatus::Completed);
}
