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
