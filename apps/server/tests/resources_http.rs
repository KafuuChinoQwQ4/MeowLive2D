mod support;
use meowlive_server::transport::http::router;
use serde_json::json;

#[tokio::test]
async fn resources_start_empty_and_reject_unknown_voice() {
    let state = support::state();
    let (code, body) =
        support::request(router(state.clone()), "GET", "/api/resources", json!({})).await;
    assert_eq!(code, 200);
    assert_eq!(body["active_voice_id"], "");
    assert_eq!(body["voices"], json!([]));
    assert_eq!(body["default_voice_available"], false);
    let (code, _) = support::request(
        router(state.clone()),
        "POST",
        "/api/voices/select",
        json!({"id":"default"}),
    )
    .await;
    assert_eq!(code, 400);
    let (code, body) = support::request(
        router(state),
        "POST",
        "/api/voices/select",
        json!({"id":"missing"}),
    )
    .await;
    assert_eq!(code, 400);
    assert_eq!(body["code"], "invalid_resource");
}

#[tokio::test]
async fn explicitly_configured_default_voice_remains_selectable() {
    let mut state = support::state();
    let config = std::sync::Arc::make_mut(&mut state.config);
    config.speech.reference_audio = "/configured/reference.wav".into();
    config.speech.prompt_text = "用户配置的参考文本".into();
    let (code, body) = support::request(
        router(state),
        "POST",
        "/api/voices/select",
        json!({"id":"default"}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(body["active_voice_id"], "default");
    assert_eq!(body["default_voice_available"], true);
}

#[tokio::test]
async fn legacy_character_with_unconfigured_default_voice_is_rejected_before_desktop_changes() {
    let state = support::state();
    let (code, body) = support::request(
        router(state.clone()),
        "POST",
        "/api/characters/save",
        json!({
            "id": null, "name": "已有角色", "model_id": "model-one",
            "voice_id": "default", "mouth_parameter": "MeowMouthOpen", "mappings": []
        }),
    )
    .await;
    assert_eq!(code, 200);
    let id = body["characters"][0]["id"].as_str().unwrap();
    let (code, body) = support::request(
        router(state.clone()),
        "POST",
        "/api/characters/select",
        json!({"id": id}),
    )
    .await;
    assert_eq!(code, 400);
    assert_eq!(body["code"], "invalid_resource");
    let (_, body) = support::request(router(state), "GET", "/api/resources", json!({})).await;
    assert_eq!(body["active_voice_id"], "");
    assert_eq!(body["active_character_id"], serde_json::Value::Null);
}

#[tokio::test]
async fn disconnected_desktop_resource_action_is_visible() {
    let (code, body) = support::request(
        router(support::state()),
        "POST",
        "/api/desktop/resources",
        json!({"type":"list_models"}),
    )
    .await;
    assert_eq!(code, 409);
    assert_eq!(body["code"], "bridge_disconnected");
}

#[tokio::test]
async fn active_alias_is_resolved_before_queueing() {
    let (state, base, task) = support::server().await;
    let (_control, _audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    let (code, body) = support::request(
        router(state),
        "POST",
        "/api/speech",
        json!({"text":"测试当前音色","voice_id":"active"}),
    )
    .await;
    assert_eq!(code, 202);
    assert_eq!(body["voice_id"], "default");
    task.abort();
}
