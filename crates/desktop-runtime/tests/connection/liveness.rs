//! Compiled within connection so tests exercise the real paired-session loop.

#[path = "../support/mod.rs"]
mod support;

use futures_util::{SinkExt, StreamExt};
use meowlive_protocol::control::ServerCommand;
use std::time::Duration;
use support::ManualDevice;
use tokio::time::{advance, timeout};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Message, protocol::Role},
};

async fn loses_one_channel(silent_control: bool) {
    let (control_client, control_server) = tokio::io::duplex(4096);
    let (audio_client, audio_server) = tokio::io::duplex(4096);
    let control = WebSocketStream::from_raw_socket(control_client, Role::Client, None).await;
    let audio = WebSocketStream::from_raw_socket(audio_client, Role::Client, None).await;
    let mut control_server =
        WebSocketStream::from_raw_socket(control_server, Role::Server, None).await;
    let mut audio_server = WebSocketStream::from_raw_socket(audio_server, Role::Server, None).await;
    let device = ManualDevice::default();
    control_server
        .send(Message::text(
            serde_json::to_string(&support::speak(1)).unwrap(),
        ))
        .await
        .unwrap();
    audio_server
        .send(Message::binary(
            support::chunk(1, 0, false).encode().unwrap(),
        ))
        .await
        .unwrap();
    let session = super::run_paired(
        control,
        audio,
        device.clone(),
        1,
        64,
        Duration::from_secs(1),
        Duration::from_secs(35),
    );
    let peer = async {
        // Keep the sibling active. Traffic there must not revive the silent channel.
        for _ in 0..4 {
            tokio::task::yield_now().await;
            advance(Duration::from_secs(10)).await;
            let active = if silent_control {
                &mut audio_server
            } else {
                &mut control_server
            };
            if active.send(Message::Ping(vec![1].into())).await.is_err() {
                break;
            }
            let _ = active.next().await;
        }
        // Keep both transports open until client deadline is observed.
        tokio::time::sleep(Duration::from_secs(60)).await;
    };
    tokio::pin!(peer);
    let result = timeout(Duration::from_secs(45), async {
        tokio::select! { result = session => result, _ = &mut peer => panic!("peer outlived test") }
    })
    .await
    .expect("client must expire the silent channel without a FIN or RST");
    let error = result.unwrap_err();
    assert!(
        error.contains(if silent_control {
            "control heartbeat"
        } else {
            "audio heartbeat"
        }),
        "{error}"
    );
    assert!(!device.0.borrow().samples.is_empty());
    assert!(device.0.borrow().stopped);
}

#[tokio::test(start_paused = true)]
async fn control_blackhole_expires_even_when_audio_keeps_sending_pings() {
    loses_one_channel(true).await;
}

#[tokio::test(start_paused = true)]
async fn audio_blackhole_expires_even_when_control_keeps_sending_pings() {
    loses_one_channel(false).await;
}

#[tokio::test(start_paused = true)]
async fn pings_refresh_both_channels_beyond_the_original_deadline() {
    let (control_client, control_server) = tokio::io::duplex(4096);
    let (audio_client, audio_server) = tokio::io::duplex(4096);
    let control = WebSocketStream::from_raw_socket(control_client, Role::Client, None).await;
    let audio = WebSocketStream::from_raw_socket(audio_client, Role::Client, None).await;
    let mut control_server =
        WebSocketStream::from_raw_socket(control_server, Role::Server, None).await;
    let mut audio_server = WebSocketStream::from_raw_socket(audio_server, Role::Server, None).await;
    let device = ManualDevice::default();
    let server = async {
        for _ in 0..5 {
            control_server
                .send(Message::Ping(vec![1].into()))
                .await
                .unwrap();
            audio_server
                .send(Message::Ping(vec![2].into()))
                .await
                .unwrap();
            assert!(matches!(
                control_server.next().await,
                Some(Ok(Message::Pong(_)))
            ));
            assert!(matches!(
                audio_server.next().await,
                Some(Ok(Message::Pong(_)))
            ));
            advance(Duration::from_secs(10)).await;
        }
        control_server
            .send(Message::text(
                serde_json::to_string(&ServerCommand::Stop { generation: 2 }).unwrap(),
            ))
            .await
            .unwrap();
        control_server.close(None).await.unwrap();
    };
    let (_, result) = tokio::join!(
        server,
        super::run_paired(
            control,
            audio,
            device,
            1,
            64,
            Duration::from_secs(1),
            Duration::from_secs(35)
        )
    );
    assert_eq!(result.unwrap_err(), "control disconnected");
}
