mod support;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn requires_audio_pair_and_rejects_duplicate_execution_client() {
    let (state, base, server) = support::server().await;
    let (mut control, _) = connect_async(format!("{base}/ws/control")).await.unwrap();
    control
        .send(Message::Text(
            json!({"type":"hello","protocol_version":meowlive_protocol::PROTOCOL_VERSION})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let hello = support::next_json(&mut control).await;
    assert_eq!(support::status(&state).await["bridge_connected"], false);
    assert!(
        connect_async(format!("{base}/ws/audio?session_id=wrong&bridge_id=wrong"))
            .await
            .is_err()
    );
    let url = format!(
        "{base}/ws/audio?session_id={}&bridge_id={}",
        hello["session_id"].as_str().unwrap(),
        hello["bridge_id"].as_str().unwrap()
    );
    let (mut audio, _) = connect_async(&url).await.unwrap();
    support::await_connected(&state, true).await;
    assert!(connect_async(&url).await.is_err());
    assert!(connect_async(format!("{base}/ws/control")).await.is_err());
    audio.close(None).await.unwrap();
    support::await_connected(&state, false).await;
    server.abort();
}

#[tokio::test]
async fn unsupported_protocol_cannot_claim_bridge() {
    let (state, base, server) = support::server().await;
    let (mut control, _) = connect_async(format!("{base}/ws/control")).await.unwrap();
    control
        .send(Message::Text(
            json!({"type":"hello","protocol_version":999})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), control.next())
        .await
        .unwrap();
    assert_eq!(support::status(&state).await["bridge_connected"], false);
    let (_control, _audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    server.abort();
}
