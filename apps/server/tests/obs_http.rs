mod support;
use meowlive_server::transport::http::router;
use serde_json::json;

#[tokio::test]
async fn obs_status_requires_a_connected_desktop_and_rejects_invalid_actions() {
    let state = support::state();
    let (code, body) = support::request(router(state.clone()), "GET", "/api/obs", json!({})).await;
    assert_eq!(code, 409, "{body}");
    for request in [
        json!({"type":"set_scene","scene_name":""}),
        json!({"type":"set_scene","scene_name":"bad\nscene"}),
        json!({"type":"start_streaming"}),
        json!({"type":"start_recording","password":"unexpected"}),
    ] {
        let (code, _) = support::request(router(state.clone()), "POST", "/api/obs", request).await;
        assert_eq!(code, 400);
    }
}

#[tokio::test]
async fn obs_status_reaches_runtime_even_when_obs_is_disabled() {
    use meowlive_desktop_runtime::{
        audio::SimulatedBackend, config::ClientConfig, connection::run_once,
    };
    let (state, base, server) = support::server().await;
    let desktop = tokio::spawn(async move {
        let config = ClientConfig {
            server_url: base,
            ..Default::default()
        };
        run_once(&config, SimulatedBackend::new(config.max_buffer_samples)).await
    });
    support::await_connected(&state, true).await;
    let (code, body) = support::request(router(state), "GET", "/api/obs", json!({})).await;
    assert_eq!(code, 200, "{body}");
    assert_eq!(
        body,
        json!({"connected":false,"recording":false,"current_scene":"","scenes":[]})
    );
    desktop.abort();
    let _ = desktop.await;
    server.abort();
}

#[tokio::test]
async fn obs_settings_save_on_execution_host_and_queries_do_not_return_passwords() {
    use meowlive_desktop_runtime::{
        audio::SimulatedBackend, config::ClientConfig, connection::run_once,
    };
    let root = std::env::temp_dir().join(format!("meowlive-obs-http-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let (state, base, server) = support::server().await;
    let mut config = ClientConfig {
        server_url: base,
        ..Default::default()
    };
    config.resolve_paths(&root.join("desktop.toml"));
    let desktop = tokio::spawn(async move {
        run_once(&config, SimulatedBackend::new(config.max_buffer_samples)).await
    });
    support::await_connected(&state, true).await;
    let (code, initial) =
        support::request(router(state.clone()), "GET", "/api/obs/settings", json!({})).await;
    assert_eq!(code, 200, "{initial}");
    assert_eq!(initial["storage_available"], true);
    let (code, saved) = support::request(router(state.clone()), "POST", "/api/obs/settings", json!({
        "enabled":false,"websocket_url":"ws://127.0.0.1:4455","password":"http-private-password","clear_password":false
    })).await;
    assert_eq!(code, 200, "{saved}");
    assert_eq!(saved["password_configured"], true);
    assert!(!saved.to_string().contains("http-private-password"));
    assert!(root.join("desktop.obs.local.json").exists());
    let (code, queried) =
        support::request(router(state.clone()), "GET", "/api/obs/settings", json!({})).await;
    assert_eq!(code, 200);
    assert_eq!(queried, saved);
    let (code, _) = support::request(router(state), "POST", "/api/obs/settings", json!({
        "enabled":true,"websocket_url":"ws://remote.example:4455","password":null,"clear_password":true
    })).await;
    assert_eq!(code, 400);
    desktop.abort();
    let _ = desktop.await;
    server.abort();
    std::fs::remove_dir_all(root).unwrap();
}
