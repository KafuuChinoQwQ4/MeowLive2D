mod support;
use futures_util::SinkExt;
use meowlive_server::transport::http::router;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn resource_response_requires_current_correlation_and_stop_stays_responsive() {
    let (state, base, server) = support::server().await;
    let (mut control, _audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    let requester = state.clone();
    let pending = tokio::spawn(async move {
        support::request(
            router(requester),
            "POST",
            "/api/desktop/resources",
            json!({"type":"list_models"}),
        )
        .await
    });
    let command = support::next_json(&mut control).await;
    assert_eq!(command["operation"]["type"], "list_models");
    control.send(Message::Text(json!({"type":"resource_result","request_id":"stale-request","result":{"type":"models","models":[]}}).to_string().into())).await.unwrap();
    let (code, error) = support::request(
        router(state.clone()),
        "POST",
        "/api/desktop/resources",
        json!({"type":"list_models"}),
    )
    .await;
    assert_eq!(code, 409);
    assert_eq!(error["code"], "resource_busy");
    control.send(Message::Text(json!({"type":"resource_result","request_id":command["request_id"],"result":{"type":"models","models":[{"id":"witch","name":"魔女"}]}}).to_string().into())).await.unwrap();
    let (code, result) = pending.await.unwrap();
    assert_eq!(code, 200);
    assert_eq!(result["models"][0]["name"], "魔女");

    let requester = state.clone();
    let pending = tokio::spawn(async move {
        support::request(
            router(requester),
            "POST",
            "/api/desktop/resources",
            json!({"type":"list_models"}),
        )
        .await
    });
    let _command = support::next_json(&mut control).await;
    let (code, _) = support::request(router(state.clone()), "POST", "/api/stop", json!({})).await;
    assert_eq!(code, 200);
    assert_eq!(pending.await.unwrap().1["code"], "resource_cancelled");
    assert_eq!(support::next_json(&mut control).await["type"], "stop");
    assert!(state.snapshot().await.bridge_connected);
    server.abort();
}

#[tokio::test]
async fn disconnect_cancels_resource_wait_instead_of_waiting_for_timeout() {
    let (state, base, server) = support::server().await;
    let (mut control, _audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    let requester = state.clone();
    let pending = tokio::spawn(async move {
        support::request(
            router(requester),
            "POST",
            "/api/desktop/resources",
            json!({"type":"import_model"}),
        )
        .await
    });
    let command = support::next_json(&mut control).await;
    assert_eq!(command["operation"]["type"], "import_model");
    control.close(None).await.unwrap();
    let (code, body) = tokio::time::timeout(std::time::Duration::from_secs(2), pending)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(code, 409);
    assert_eq!(body["code"], "bridge_disconnected");
    server.abort();
}

#[tokio::test]
async fn invalid_desktop_result_is_rejected_without_publishing_it() {
    let (state, base, server) = support::server().await;
    let (mut control, _audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    let pending = tokio::spawn(async move {
        support::request(
            router(state),
            "POST",
            "/api/desktop/resources",
            json!({"type":"list_models"}),
        )
        .await
    });
    let command = support::next_json(&mut control).await;
    control.send(Message::Text(json!({"type":"resource_result","request_id":command["request_id"],"result":{"type":"models","models":[{"id":"","name":"bad"}]}}).to_string().into())).await.unwrap();
    let (code, _) = pending.await.unwrap();
    assert_eq!(code, 502);
    server.abort();
}
