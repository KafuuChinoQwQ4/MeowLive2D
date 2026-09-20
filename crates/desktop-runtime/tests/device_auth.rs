use futures_util::SinkExt;
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        Message,
        handshake::server::{Request, Response},
    },
};

fn private_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "meowlive-auth-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

async fn invalid_device_credential(contents: Option<&[u8]>) -> String {
    let root = private_root();
    std::fs::create_dir_all(&root).unwrap();
    if let Some(contents) = contents {
        std::fs::write(root.join("device.txt"), contents).unwrap();
    }
    let mut config =
        ClientConfig::from_toml("server_url='http://127.0.0.1:9'\ndevice_token_file='device.txt'")
            .unwrap();
    config.resolve_paths(&root.join("desktop.toml"));
    let error = run_once(&config, SimulatedBackend::new(48_000))
        .await
        .unwrap_err();
    std::fs::remove_dir_all(root).unwrap();
    error
}

#[tokio::test]
async fn missing_device_credential_file_is_rejected_before_connecting() {
    assert_eq!(
        invalid_device_credential(None).await,
        "cannot read private device credential"
    );
}

#[tokio::test]
async fn short_device_credential_is_rejected_before_connecting() {
    assert_eq!(
        invalid_device_credential(Some(b"too-short")).await,
        "invalid private device credential"
    );
}

#[tokio::test]
async fn oversized_device_credential_is_rejected_before_connecting() {
    let credential = vec![b'x'; 513];
    assert_eq!(
        invalid_device_credential(Some(&credential)).await,
        "invalid private device credential"
    );
}

#[tokio::test]
async fn non_ascii_device_credential_is_rejected_before_connecting() {
    let credential = "凭据".repeat(16);
    assert_eq!(
        invalid_device_credential(Some(credential.as_bytes())).await,
        "invalid private device credential"
    );
}

#[tokio::test]
#[allow(clippy::result_large_err)]
async fn private_device_credential_is_sent_on_both_websocket_channels() {
    let root = private_root();
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("device.txt"),
        "independent-device-credential-123456789\n",
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let headers = captured.clone();
    let config_text = format!(
        "server_url='http://{}'\ndevice_token_file='device.txt'",
        listener.local_addr().unwrap()
    );
    let mut config = ClientConfig::from_toml(&config_text).unwrap();
    config.resolve_paths(&root.join("desktop.toml"));
    let peer = tokio::spawn(async move {
        let read = move |request: &Request, response: Response| {
            headers.lock().unwrap().push(
                request
                    .headers()
                    .get("authorization")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            );
            Ok(response)
        };
        let (stream, _) = listener.accept().await.unwrap();
        let mut control = accept_hdr_async(stream, read.clone()).await.unwrap();
        control.send(Message::text(format!(r#"{{"type":"hello","protocol_version":{},"session_id":"s","bridge_id":"b","generation":0}}"#, meowlive_protocol::PROTOCOL_VERSION))).await.unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let mut audio = accept_hdr_async(stream, read).await.unwrap();
        audio.close(None).await.unwrap();
        control.close(None).await.unwrap();
    });
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        run_once(&config, SimulatedBackend::new(48000)),
    )
    .await
    .unwrap();
    assert!(result.is_err());
    peer.await.unwrap();
    assert_eq!(
        *captured.lock().unwrap(),
        vec!["Bearer independent-device-credential-123456789"; 2]
    );
    std::fs::remove_dir_all(root).unwrap();
}
