mod support;

use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use meowlive_server::{transport::http::router, worker::run_worker};
use serde_json::json;

#[tokio::test]
async fn real_runtime_connects_receives_pcm_and_reports_completion() {
    let (state, base, server) = support::server().await;
    let config = ClientConfig {
        server_url: base.replacen("ws://", "http://", 1),
        ..ClientConfig::default()
    };
    let client = tokio::spawn(async move {
        run_once(&config, SimulatedBackend::new(config.max_buffer_samples)).await
    });
    support::await_connected(&state, true).await;
    let worker = tokio::spawn(run_worker(state.clone()));
    let (code, _) = support::request(
        router(state.clone()),
        "POST",
        "/api/speech",
        json!({"text":"跨端闭环","voice_id":"default"}),
    )
    .await;
    assert_eq!(code, 202);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if support::status(&state).await["speeches"][0]["status"] == "completed" {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    state.shutdown().await;
    support::await_connected(&state, false).await;
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(2), client)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    worker.abort();
    server.abort();
}
