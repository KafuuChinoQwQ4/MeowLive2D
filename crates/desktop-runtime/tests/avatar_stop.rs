mod avatar_support;
use avatar_support::*;
use futures_util::{SinkExt, StreamExt};
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once, lip_sync::LipSyncConfig,
    presentation::AvatarDriver,
};
use meowlive_protocol::{
    audio::{AudioChunk, AudioFormat},
    control::{ClientMessage, ServerCommand},
    execution::ExecutionStatus,
};
use serde_json::json;
use std::time::Duration;
use tokio::{net::TcpListener, sync::oneshot, time::timeout};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[tokio::test]
async fn stalled_vts_response_does_not_delay_stop_receipt_or_zero_reset() {
    let mut fixture = Fixture::new(true).await;
    fixture.config.request_timeout_ms = 2000;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = ClientConfig::from_toml(&format!(
        "server_url='http://{}'",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let driver = AvatarDriver::start(fixture.config.clone()).unwrap();
    let backend = driver
        .observe(
            SimulatedBackend::new(config.max_buffer_samples),
            LipSyncConfig::default(),
        )
        .unwrap();
    let client = tokio::spawn(async move { run_once(&config, backend).await });
    let (stop, stopped) = oneshot::channel();
    let (close, closed) = oneshot::channel();
    let server = tokio::spawn(async move {
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
        let speak = ServerCommand::Speak {
            utterance_id: "u1".into(),
            generation: 1,
            format: AudioFormat {
                sample_rate: 24000,
                channels: 1,
            },
        };
        control
            .send(Message::text(serde_json::to_string(&speak).unwrap()))
            .await
            .unwrap();
        for sequence in 0..4 {
            let chunk = AudioChunk {
                utterance_id: "u1".into(),
                generation: 1,
                sequence,
                samples: vec![16000; 12000],
                end: sequence == 3,
            };
            audio
                .send(Message::binary(chunk.encode().unwrap()))
                .await
                .unwrap();
        }
        stopped.await.unwrap();
        let command = ServerCommand::Stop { generation: 2 };
        control
            .send(Message::text(serde_json::to_string(&command).unwrap()))
            .await
            .unwrap();
        timeout(Duration::from_millis(300), async {
            loop {
                let message = control.next().await.unwrap().unwrap();
                if let ClientMessage::Receipt { receipt } =
                    serde_json::from_str(message.to_text().unwrap()).unwrap()
                {
                    if receipt.status == ExecutionStatus::Cancelled {
                        assert_eq!(receipt.utterance_id, "u1");
                        break;
                    }
                }
            }
        })
        .await
        .expect("Stop must not wait for the 2 second VTS response timeout");
        closed.await.unwrap();
        control.close(None).await.unwrap();
    });
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    timeout(Duration::from_secs(1), async {
        loop {
            let req = request(&mut socket, "InjectParameterDataRequest").await;
            if req["data"]["parameterValues"][0]["value"].as_f64().unwrap() > 0.0 {
                break; // Leave this nonzero request unacknowledged.
            }
            respond(&mut socket, &req, "InjectParameterDataResponse", json!({})).await;
        }
    })
    .await
    .unwrap();
    stop.send(()).unwrap();
    assert_eq!(
        timeout(Duration::from_millis(300), injected(&mut socket))
            .await
            .expect("Stop must supersede the pending mouth value"),
        0.0
    );
    close.send(()).unwrap();
    server.await.unwrap();
    assert!(client.await.unwrap().is_err());
    // Keep acknowledging shutdown resets while the driver finishes.
    tokio::select! {
        _ = driver.shutdown() => {},
        _ = async { loop { assert_eq!(injected(&mut socket).await, 0.0); } } => unreachable!(),
    }
}
