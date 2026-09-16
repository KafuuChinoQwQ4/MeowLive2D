mod support;
use axum::{
    Router,
    extract::ws::{Message, WebSocketUpgrade},
    routing::get,
};
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, avatar::VtsConfig, config::ClientConfig,
    connection::run_once_with_mouth, presentation::AvatarDriver,
};
use meowlive_server::transport::http::router;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Mutex, Notify};

struct Files(PathBuf);
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn panel_resource_commands_reach_vts_and_slow_vts_does_not_delay_stop() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/resources-runtime")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root).unwrap();
    let _files = Files(root.clone());
    let token = root.join("token.json");
    std::fs::write(
        &token,
        r#"{"authenticationToken":"controlled-resource-token"}"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let calls = Arc::new(Mutex::new(Vec::<Value>::new()));
    let block = Arc::new(AtomicBool::new(false));
    let blocked = Arc::new(Notify::new());
    let (seen, hold, notification) = (calls.clone(), block.clone(), blocked.clone());
    let vts=Router::new().route("/",get(move|upgrade:WebSocketUpgrade|{
        let (seen,hold,notification)=(seen.clone(),hold.clone(),notification.clone());
        async move { upgrade.on_upgrade(move|mut socket|async move{
            while let Some(Ok(Message::Text(text)))=socket.recv().await{
                let request:Value=serde_json::from_str(&text).unwrap();
                let kind=request["messageType"].as_str().unwrap();
                if kind!="InjectParameterDataRequest"{seen.lock().await.push(request.clone());}
                if kind=="AvailableModelsRequest" && hold.load(Ordering::Acquire){notification.notify_one();tokio::time::sleep(Duration::from_secs(3)).await;}
                let data=match kind {
                    "AuthenticationRequest"=>json!({"authenticated":true}),
                    "ParameterCreationRequest"=>json!({"parameterName":request["data"]["parameterName"]}),
                    "InjectParameterDataRequest"=>json!({}),
                    "AvailableModelsRequest"=>json!({"numberOfModels":1,"availableModels":[{"modelLoaded":true,"modelName":"魔女","modelID":"witch"}]}),
                    "ModelLoadRequest"=>json!({"modelID":request["data"]["modelID"]}),
                    "HotkeysInCurrentModelRequest"=>json!({"modelLoaded":true,"modelName":"魔女","modelID":"witch","availableHotkeys":[{"name":"开心","type":"ToggleExpression","hotkeyID":"fallback-smile"}]}),
                    "CurrentModelRequest"=>json!({"modelLoaded":true,"modelID":"witch"}),
                    "HotkeyTriggerRequest"=>json!({"hotkeyID":request["data"]["hotkeyID"]}),
                    _=>panic!("unexpected VTS request {kind}"),
                };
                let response=json!({"apiName":"VTubeStudioPublicAPI","apiVersion":"1.0","requestID":request["requestID"],"messageType":kind.replace("Request","Response"),"data":data});
                if socket.send(Message::Text(response.to_string().into())).await.is_err(){break;}
            }
        }) }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let vts_url = format!("ws://{}/", listener.local_addr().unwrap());
    let vts_server = tokio::spawn(async {
        axum::serve(listener, vts).await.unwrap();
    });
    let (mut state, base, server) = support::server().await;
    Arc::make_mut(&mut state.config).speech.reference_audio = "/configured/reference.wav".into();
    let config = ClientConfig {
        server_url: base,
        vtube_studio: VtsConfig {
            enabled: true,
            websocket_url: vts_url,
            token_path: token,
            ..Default::default()
        },
        ..Default::default()
    };
    let driver = AvatarDriver::start(config.vtube_studio.clone()).unwrap();
    let mouth = driver.mouth_parameter();
    let desktop = tokio::spawn(async move {
        run_once_with_mouth(
            &config,
            SimulatedBackend::new(config.max_buffer_samples),
            Some(mouth),
        )
        .await
    });
    support::await_connected(&state, true).await;
    let (code, models) = support::request(
        router(state.clone()),
        "POST",
        "/api/desktop/resources",
        json!({"type":"list_models"}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(models["models"][0]["name"], "魔女");
    let (code,saved)=support::request(router(state.clone()),"POST","/api/characters/save",json!({"id":null,"name":"魔女","model_id":"witch","voice_id":"default","mouth_parameter":"WitchMouth","mappings":[{"intent":"开心","hotkey_id":"missing-primary","fallback_hotkey_id":"fallback-smile","validated":false}]})).await;
    assert_eq!(code, 200);
    let id = saved["characters"][0]["id"].clone();
    let (code, snapshot) = support::request(
        router(state.clone()),
        "POST",
        "/api/characters/select",
        json!({"id":id}),
    )
    .await;
    assert_eq!(code, 200, "{snapshot}");
    assert_eq!(snapshot["active_character_id"], id);
    let (code, snapshot) = support::request(
        router(state.clone()),
        "POST",
        "/api/characters/preview",
        json!({"character_id":id,"intent":"开心"}),
    )
    .await;
    assert_eq!(code, 200, "{snapshot}");
    assert_eq!(snapshot["characters"][0]["mappings"][0]["validated"], true);
    let observed = calls.lock().await;
    assert!(
        observed
            .iter()
            .any(|r| r["messageType"] == "ModelLoadRequest" && r["data"]["modelID"] == "witch")
    );
    assert!(
        observed
            .iter()
            .any(|r| r["messageType"] == "ParameterCreationRequest"
                && r["data"]["parameterName"] == "WitchMouth")
    );
    assert!(
        observed
            .iter()
            .any(|r| r["messageType"] == "HotkeyTriggerRequest"
                && r["data"]["hotkeyID"] == "fallback-smile")
    );
    drop(observed);
    block.store(true, Ordering::Release);
    let request_state = state.clone();
    let pending = tokio::spawn(async move {
        support::request(
            router(request_state),
            "POST",
            "/api/desktop/resources",
            json!({"type":"list_models"}),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), blocked.notified())
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), state.stop())
        .await
        .unwrap();
    assert!(result.bridge_connected);
    assert_eq!(pending.await.unwrap().1["code"], "resource_cancelled");
    desktop.abort();
    let _ = desktop.await;
    driver.shutdown().await;
    server.abort();
    vts_server.abort();
}
