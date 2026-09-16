mod avatar_support;

use avatar_support::{Fixture, request, respond};
use meowlive_desktop_runtime::avatar::{ResourceError, VtsResources};
use serde_json::json;

#[tokio::test]
async fn lists_loads_and_previews_resources_through_authenticated_vts_api() {
    let fixture = Fixture::new(true).await;
    let config = fixture.config.clone();
    let connecting = tokio::spawn(async move { VtsResources::connect(&config).await });
    let mut socket = fixture.accept().await;
    let auth = request(&mut socket, "AuthenticationRequest").await;
    respond(
        &mut socket,
        &auth,
        "AuthenticationResponse",
        json!({"authenticated":true}),
    )
    .await;
    let mut resources = connecting.await.unwrap().unwrap();

    let server = tokio::spawn(async move {
        let models = request(&mut socket, "AvailableModelsRequest").await;
        assert_eq!(models["data"], json!({}));
        respond(
            &mut socket,
            &models,
            "AvailableModelsResponse",
            json!({"numberOfModels":1,"availableModels":[{
                "modelLoaded":false,"modelName":"魔女","modelID":"model-1",
                "vtsModelName":"model.vtube.json","vtsModelIconName":"icon.png"
            }]}),
        )
        .await;

        let load = request(&mut socket, "ModelLoadRequest").await;
        assert_eq!(load["data"], json!({"modelID":"model-1"}));
        respond(
            &mut socket,
            &load,
            "ModelLoadResponse",
            json!({"modelID":"model-1"}),
        )
        .await;

        let hotkeys = request(&mut socket, "HotkeysInCurrentModelRequest").await;
        assert_eq!(hotkeys["data"], json!({"modelID":"model-1"}));
        respond(
            &mut socket,
            &hotkeys,
            "HotkeysInCurrentModelResponse",
            json!({"modelLoaded":true,"modelName":"魔女","modelID":"model-1",
                "availableHotkeys":[{"name":"开心","type":"ToggleExpression",
                "description":"happy","file":"happy.exp3.json","hotkeyID":"hotkey-1",
                "keyCombination":[],"onScreenButtonID":-1}]}),
        )
        .await;

        let current = request(&mut socket, "CurrentModelRequest").await;
        respond(
            &mut socket,
            &current,
            "CurrentModelResponse",
            json!({"modelLoaded":true,"modelID":"model-1","modelName":"魔女"}),
        )
        .await;
        let trigger = request(&mut socket, "HotkeyTriggerRequest").await;
        assert_eq!(trigger["data"], json!({"hotkeyID":"hotkey-1"}));
        respond(
            &mut socket,
            &trigger,
            "HotkeyTriggerResponse",
            json!({"hotkeyID":"hotkey-1"}),
        )
        .await;
    });

    let models = resources.available_models().await.unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "model-1");
    assert_eq!(models[0].name, "魔女");
    assert!(!models[0].loaded);
    resources.load_model("model-1").await.unwrap();
    let hotkeys = resources.hotkeys(Some("model-1")).await.unwrap();
    assert_eq!(hotkeys.len(), 1);
    assert_eq!(hotkeys[0].id, "hotkey-1");
    assert_eq!(hotkeys[0].name, "开心");
    assert_eq!(hotkeys[0].kind, "ToggleExpression");
    resources
        .preview_hotkey("model-1", "hotkey-1")
        .await
        .unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn refuses_preview_after_user_switches_to_another_model() {
    let fixture = Fixture::new(true).await;
    let config = fixture.config.clone();
    let connecting = tokio::spawn(async move { VtsResources::connect(&config).await });
    let mut socket = fixture.accept().await;
    let auth = request(&mut socket, "AuthenticationRequest").await;
    respond(
        &mut socket,
        &auth,
        "AuthenticationResponse",
        json!({"authenticated":true}),
    )
    .await;
    let mut resources = connecting.await.unwrap().unwrap();
    let server = tokio::spawn(async move {
        let current = request(&mut socket, "CurrentModelRequest").await;
        respond(
            &mut socket,
            &current,
            "CurrentModelResponse",
            json!({"modelLoaded":true,"modelID":"other-model","modelName":"other"}),
        )
        .await;
    });

    assert_eq!(
        resources
            .preview_hotkey("expected-model", "hotkey-1")
            .await
            .unwrap_err(),
        ResourceError::CurrentModelChanged
    );
    server.await.unwrap();
}

#[tokio::test]
async fn resource_connection_never_opens_authorization_without_cached_token() {
    let fixture = Fixture::new(false).await;
    assert_eq!(
        VtsResources::connect(&fixture.config).await.err().unwrap(),
        ResourceError::AuthorizationRequired
    );
}

#[tokio::test]
async fn rejects_a_model_load_response_for_another_model() {
    let fixture = Fixture::new(true).await;
    let config = fixture.config.clone();
    let connecting = tokio::spawn(async move { VtsResources::connect(&config).await });
    let mut socket = fixture.accept().await;
    let auth = request(&mut socket, "AuthenticationRequest").await;
    respond(
        &mut socket,
        &auth,
        "AuthenticationResponse",
        json!({"authenticated":true}),
    )
    .await;
    let mut resources = connecting.await.unwrap().unwrap();
    let server = tokio::spawn(async move {
        let load = request(&mut socket, "ModelLoadRequest").await;
        respond(
            &mut socket,
            &load,
            "ModelLoadResponse",
            json!({"modelID":"other-model"}),
        )
        .await;
    });

    assert_eq!(
        resources.load_model("expected-model").await.unwrap_err(),
        ResourceError::UnexpectedModel
    );
    server.await.unwrap();
}
