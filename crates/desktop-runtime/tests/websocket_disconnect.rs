mod support;

use futures_util::{SinkExt, StreamExt};
use meowlive_desktop_runtime::{config::ClientConfig, connection::run_once};
use meowlive_protocol::{audio::AudioChunk, control::ServerCommand};
use std::time::Duration;
use support::{ManualDevice, speak};
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[tokio::test]
async fn losing_audio_socket_stops_an_active_device_and_closes_control() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = ClientConfig::from_toml(&format!(
        "server_url='http://{}'",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let device = ManualDevice::default();
    let server_device = device.clone();
    let server = async {
        let (stream, _) = listener.accept().await.unwrap();
        let mut control = accept_async(stream).await.unwrap();
        control.next().await.unwrap().unwrap();
        let hello = ServerCommand::Hello {
            protocol_version: meowlive_protocol::PROTOCOL_VERSION,
            session_id: "s1".into(),
            bridge_id: "b1".into(),
            generation: 1,
        };
        control
            .send(Message::text(serde_json::to_string(&hello).unwrap()))
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let mut audio = accept_async(stream).await.unwrap();
        control
            .send(Message::text(serde_json::to_string(&speak(1)).unwrap()))
            .await
            .unwrap();
        let chunk: AudioChunk = support::chunk(1, 0, false);
        audio
            .send(Message::binary(chunk.encode().unwrap()))
            .await
            .unwrap();
        timeout(Duration::from_secs(1), async {
            while server_device.0.borrow().samples.is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        audio.close(None).await.unwrap();
        assert!(!matches!(control.next().await, Some(Ok(Message::Text(_)))));
    };
    let (_, result) = timeout(Duration::from_secs(2), async {
        tokio::join!(server, run_once(&config, device.clone()))
    })
    .await
    .unwrap();
    assert!(result.is_err());
    assert!(device.0.borrow().stopped);
}
