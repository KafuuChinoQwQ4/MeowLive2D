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

#[tokio::test]
async fn model_deletion_checks_role_references_and_fences_speech_until_confirmation() {
    let (state, base, server) = support::server().await;
    let (mut control, _audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    state
        .resources
        .create_character(
            meowlive_domain::character::CharacterProfile::new(
                "role",
                "猫咪",
                "model-cat",
                "default",
                "MeowMouthOpen",
                vec![],
            )
            .unwrap(),
        )
        .unwrap();
    let id = "a".repeat(64);
    for linked in [true, false] {
        if !linked {
            state.resources.delete_character("role").unwrap();
        }
        let requester = state.clone();
        let request_id = id.clone();
        let pending = tokio::spawn(async move {
            support::request(
                router(requester),
                "POST",
                "/api/desktop/resources",
                json!({"type":"delete_imported_model","id":request_id}),
            )
            .await
        });
        let list = support::next_json(&mut control).await;
        assert_eq!(list["operation"]["type"], "list_imported_models");
        control.send(Message::Text(json!({"type":"resource_result","request_id":list["request_id"],
            "result":{"type":"imported_models","models":[{"id":id,"name":"猫咪","model_id":"model-cat"}]}}).to_string().into())).await.unwrap();
        if linked {
            let (code, error) = pending.await.unwrap();
            assert_eq!(code, 400);
            assert!(error["message"].as_str().unwrap().contains("角色配置"));
        } else {
            let command = support::next_json(&mut control).await;
            assert_eq!(
                command["operation"],
                json!({"type":"delete_imported_model","id":id})
            );
            let (code, _) = support::request(
                router(state.clone()),
                "POST",
                "/api/speech",
                json!({"text":"删除期间不能播报","voice_id":"default"}),
            )
            .await;
            assert_eq!(code, 409);
            control
                .send(Message::Text(
                    json!({"type":"resource_result","request_id":command["request_id"],
                "result":{"type":"model_deleted","id":id,"restart_required":true}})
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let (code, result) = pending.await.unwrap();
            assert_eq!(code, 200);
            assert_eq!(result["id"], id);
        }
    }
    server.abort();
}
