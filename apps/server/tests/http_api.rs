mod support;
use meowlive_server::transport::http::router;
use serde_json::json;

#[tokio::test]
async fn status_is_versioned_and_offline_bridge_rejects_speech() {
    let state = support::state();
    let status = support::status(&state).await;
    assert_eq!(
        status["protocol_version"],
        meowlive_protocol::PROTOCOL_VERSION
    );
    assert_eq!(status["bridge_connected"], false);
    assert_eq!(status["speeches"], json!([]));
    let (code, error) = support::request(
        router(state),
        "POST",
        "/api/speech",
        json!({"text":"你好","voice_id":"default"}),
    )
    .await;
    assert_eq!(code, 409);
    assert_eq!(error["code"], "bridge_disconnected");
}

#[tokio::test]
async fn malformed_input_returns_structured_error() {
    let (code, body) = support::request(
        router(support::state()),
        "POST",
        "/api/speech",
        json!({"text":42}),
    )
    .await;
    assert_eq!(code, 400);
    assert_eq!(body["code"], "invalid_request");
}

#[tokio::test]
async fn stop_advances_generation_without_requiring_device() {
    let state = support::state();
    let (code, body) =
        support::request(router(state.clone()), "POST", "/api/stop", json!({})).await;
    assert_eq!(code, 200);
    assert_eq!(body["generation"], 1);
}
