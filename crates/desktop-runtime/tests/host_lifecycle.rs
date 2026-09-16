use futures_util::{SinkExt, StreamExt};
use meowlive_desktop_runtime::{config::ClientConfig, host::RuntimeHandle};
use meowlive_protocol::{PROTOCOL_VERSION, control::ServerCommand};
use std::time::{Duration, Instant};
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[test]
fn startup_reports_avatar_initialization_errors_synchronously() {
    let mut config = ClientConfig::default();
    config.vtube_studio.mouth_parameter.clear();

    RuntimeHandle::start(config, true)
        .err()
        .expect("invalid avatar configuration must fail host startup");
}

#[test]
fn shutdown_interrupts_offline_retry_without_waiting_for_reconnect_delay() {
    let config =
        ClientConfig::from_toml("server_url='http://127.0.0.1:1'\nreconnect_delay_ms=60000")
            .unwrap();
    let mut runtime = RuntimeHandle::start(config, true).unwrap();
    let started = Instant::now();
    runtime.shutdown().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!runtime.status().running);
    runtime.shutdown().unwrap();
}

#[tokio::test]
async fn dropping_host_closes_both_paired_connections() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = ClientConfig::from_toml(&format!(
        "server_url='http://{}'",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let runtime = RuntimeHandle::start(config, true).unwrap();
    timeout(Duration::from_secs(3), async {
        let (stream, _) = listener.accept().await.unwrap();
        let mut control = accept_async(stream).await.unwrap();
        control.next().await.unwrap().unwrap();
        let hello = ServerCommand::Hello {
            protocol_version: PROTOCOL_VERSION,
            session_id: "host".into(),
            bridge_id: "host-bridge".into(),
            generation: 1,
        };
        control
            .send(Message::text(serde_json::to_string(&hello).unwrap()))
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let mut audio = accept_async(stream).await.unwrap();
        drop(runtime);
        assert!(!matches!(control.next().await, Some(Ok(Message::Text(_)))));
        assert!(!matches!(audio.next().await, Some(Ok(Message::Binary(_)))));
    })
    .await
    .unwrap();
}
