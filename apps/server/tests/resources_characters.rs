mod support;
use futures_util::SinkExt;
use meowlive_server::{state::AppState, transport::http::router};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

async fn request(state: AppState, path: &'static str, body: Value) -> (u16, Value) {
    support::request(router(state), "POST", path, body).await
}
async fn reply(control: &mut support::Socket, command: &Value, result: Value) {
    control
        .send(Message::Text(
            json!({"type":"resource_result","request_id":command["request_id"],"result":result})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
}

#[tokio::test]
async fn character_selection_waits_for_model_and_preview_validation_cannot_be_forged() {
    let (mut state, base, server) = support::server().await;
    std::sync::Arc::make_mut(&mut state.config)
        .speech
        .reference_audio = "/configured/reference.wav".into();
    let (mut control, _audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    let mut profile = json!({"id":null,"name":"魔女","model_id":"witch","voice_id":"default","mouth_parameter":"WitchMouth","mappings":[{"intent":"开心","hotkey_id":"smile","fallback_hotkey_id":"wave","validated":true}]});
    let (code, saved) = request(state.clone(), "/api/characters/save", profile.clone()).await;
    assert_eq!(code, 200);
    assert_eq!(saved["characters"][0]["mappings"][0]["validated"], false);
    let id = saved["characters"][0]["id"].clone();
    profile["id"] = id.clone();
    let selected = tokio::spawn(request(
        state.clone(),
        "/api/characters/select",
        json!({"id":id}),
    ));
    let command = support::next_json(&mut control).await;
    assert_eq!(
        command["operation"],
        json!({"type":"load_model","model_id":"witch","mouth_parameter":"WitchMouth"})
    );
    assert_eq!(state.resources.snapshot().active_character_id, None);
    let (code, error) = request(
        state.clone(),
        "/api/speech",
        json!({"text":"切换中","voice_id":"active"}),
    )
    .await;
    assert_eq!(code, 409);
    assert_eq!(error["code"], "resource_busy");
    reply(
        &mut control,
        &command,
        json!({"type":"model_loaded","model_id":"witch"}),
    )
    .await;
    let (code, snapshot) = selected.await.unwrap();
    assert_eq!(code, 200);
    assert_eq!(snapshot["active_character_id"], id);

    let preview = tokio::spawn(request(
        state.clone(),
        "/api/characters/preview",
        json!({"character_id":id,"intent":"开心"}),
    ));
    let command = support::next_json(&mut control).await;
    assert_eq!(
        command["operation"],
        json!({"type":"trigger_hotkey","model_id":"witch","hotkey_id":"smile","fallback_hotkey_id":"wave"})
    );
    reply(
        &mut control,
        &command,
        json!({"type":"hotkey_triggered","hotkey_id":"wave"}),
    )
    .await;
    let (code, snapshot) = preview.await.unwrap();
    assert_eq!(code, 200);
    assert_eq!(snapshot["characters"][0]["mappings"][0]["validated"], true);
    profile["mappings"][0]["hotkey_id"] = json!("new-smile");
    let (code, snapshot) = request(state.clone(), "/api/characters/save", profile).await;
    assert_eq!(code, 200);
    assert_eq!(snapshot["characters"][0]["mappings"][0]["validated"], false);
    let preview = tokio::spawn(request(
        state.clone(),
        "/api/characters/preview",
        json!({"character_id":id,"intent":"开心"}),
    ));
    let command = support::next_json(&mut control).await;
    reply(
        &mut control,
        &command,
        json!({"type":"hotkey_triggered","hotkey_id":"stale-smile"}),
    )
    .await;
    assert_eq!(preview.await.unwrap().0, 502);
    assert!(!state.resources.snapshot().characters[0].mappings[0].validated());
    server.abort();
}

#[tokio::test]
async fn selecting_unknown_character_never_sends_desktop_commands() {
    let (code, error) = request(
        support::state(),
        "/api/characters/select",
        json!({"id":"missing"}),
    )
    .await;
    assert_eq!(code, 400);
    assert_eq!(error["code"], "invalid_resource");
}
