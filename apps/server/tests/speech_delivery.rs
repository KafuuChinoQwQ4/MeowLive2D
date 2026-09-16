mod support;
use futures_util::SinkExt;
use meowlive_server::{transport::http::router, worker::run_worker};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn completion_requires_device_receipt_after_pcm_delivery() {
    let (state, base, server) = support::server().await;
    let (mut control, mut audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    let worker = tokio::spawn(run_worker(state.clone()));
    let (code, accepted) = support::request(
        router(state.clone()),
        "POST",
        "/api/speech",
        json!({"text":"你好","voice_id":"default"}),
    )
    .await;
    assert_eq!(code, 202);
    let speak = support::next_json(&mut control).await;
    assert_eq!(speak["utterance_id"], accepted["id"]);
    let frame = support::next_data(&mut audio).await;
    let chunk = meowlive_protocol::audio::AudioChunk::decode(&frame.into_data()).unwrap();
    assert_eq!(chunk.samples, [1, -1, 0, 0]);
    assert!(chunk.end);
    assert_eq!(
        support::status(&state).await["speeches"][0]["status"],
        "ready"
    );
    for status in ["started", "completed"] {
        control.send(Message::Text(json!({"type":"receipt","receipt":{"utterance_id":accepted["id"],"generation":accepted["generation"],"status":status,"error":null}}).to_string().into())).await.unwrap();
    }
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while support::status(&state).await["speeches"][0]["status"] != "completed" {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    worker.abort();
    server.abort();
}

#[tokio::test]
async fn stop_cancels_queued_speech_and_uses_independent_control_connection() {
    let (state, base, server) = support::server().await;
    let (mut control, _audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    let (_, accepted) = support::request(
        router(state.clone()),
        "POST",
        "/api/speech",
        json!({"text":"待播","voice_id":"default"}),
    )
    .await;
    let (_, stopped) =
        support::request(router(state.clone()), "POST", "/api/stop", json!({})).await;
    assert!(stopped["generation"].as_u64() > accepted["generation"].as_u64());
    assert_eq!(stopped["speeches"][0]["status"], "cancelled");
    let command = support::next_json(&mut control).await;
    assert_eq!(command["type"], "stop");
    assert_eq!(command["generation"], stopped["generation"]);
    server.abort();
}
