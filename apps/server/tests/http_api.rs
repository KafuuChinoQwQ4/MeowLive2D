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

#[tokio::test]
async fn speech_sentence_parallelism_is_bounded_and_shared_between_clients() {
    let state = support::state();
    let (code, body) = support::request(
        router(state.clone()),
        "GET",
        "/api/speech/settings",
        json!({}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(body["sentence_batch_size"], 4);
    for value in [1, 16] {
        let (code, body) = support::request(
            router(state.clone()),
            "POST",
            "/api/speech/settings",
            json!({"sentence_batch_size":value}),
        )
        .await;
        assert_eq!(code, 200);
        assert_eq!(body["sentence_batch_size"], value);
        let (_, body) = support::request(
            router(state.clone()),
            "GET",
            "/api/speech/settings",
            json!({}),
        )
        .await;
        assert_eq!(body["sentence_batch_size"], value);
    }
    for value in [json!(0), json!(17), json!(1.5), json!("4")] {
        let (code, _) = support::request(
            router(state.clone()),
            "POST",
            "/api/speech/settings",
            json!({"sentence_batch_size":value}),
        )
        .await;
        assert_eq!(code, 400);
    }
    let (_, body) = support::request(router(state), "GET", "/api/speech/settings", json!({})).await;
    assert_eq!(body["sentence_batch_size"], 16);
}
