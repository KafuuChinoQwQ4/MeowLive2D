use futures_util::{SinkExt, StreamExt};
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use meowlive_protocol::{
    audio::{AudioChunk, AudioFormat},
    control::{ClientMessage, ServerCommand},
    execution::ExecutionStatus,
};
use tokio::{
    net::TcpListener,
    time::{Duration, timeout},
};
use tokio_tungstenite::{
    accept_async, accept_hdr_async,
    tungstenite::{
        Message,
        handshake::server::{Request, Response},
    },
};

#[tokio::test]
#[allow(clippy::result_large_err)] // tungstenite's handshake callback fixes the error type.
async fn paired_websockets_receive_audio_and_report_only_after_simulated_playout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = ClientConfig::from_toml(&format!(
        "server_url='http://{}'",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let server = async {
        let (stream, _) = listener.accept().await.unwrap();
        let mut control = accept_async(stream).await.unwrap();
        let hello = control.next().await.unwrap().unwrap().into_text().unwrap();
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(&hello).unwrap(),
            ClientMessage::Hello {
                protocol_version: meowlive_protocol::PROTOCOL_VERSION
            }
        ));
        let hello = ServerCommand::Hello {
            protocol_version: meowlive_protocol::PROTOCOL_VERSION,
            session_id: "s1".into(),
            bridge_id: "b1".into(),
            generation: 7,
        };
        control
            .send(Message::text(serde_json::to_string(&hello).unwrap()))
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let mut audio = accept_hdr_async(stream, |request: &Request, response: Response| {
            assert_eq!(
                request.uri().to_string(),
                "/ws/audio?session_id=s1&bridge_id=b1"
            );
            Ok(response)
        })
        .await
        .unwrap();
        let speak = ServerCommand::Speak {
            utterance_id: "u1".into(),
            generation: 7,
            format: AudioFormat {
                sample_rate: 24_000,
                channels: 1,
            },
        };
        // The sockets have no cross-connection ordering guarantee.
        let chunk = AudioChunk {
            utterance_id: "u1".into(),
            generation: 7,
            sequence: 0,
            samples: vec![3; 2_400],
            end: true,
        };
        audio
            .send(Message::binary(chunk.encode().unwrap()))
            .await
            .unwrap();
        let start = std::time::Instant::now();
        control
            .send(Message::text(serde_json::to_string(&speak).unwrap()))
            .await
            .unwrap();
        let mut statuses = Vec::new();
        while statuses.len() < 2 {
            let msg = control.next().await.unwrap().unwrap().into_text().unwrap();
            if let ClientMessage::Receipt { receipt } = serde_json::from_str(&msg).unwrap() {
                statuses.push(receipt.status);
            }
        }
        assert_eq!(
            statuses,
            vec![ExecutionStatus::Started, ExecutionStatus::Completed]
        );
        assert!(start.elapsed() >= Duration::from_millis(100));
        audio.close(None).await.unwrap();
    };
    let client = run_once(&config, SimulatedBackend::new(config.max_buffer_samples));
    let (_, result) = timeout(Duration::from_secs(3), async {
        tokio::join!(server, client)
    })
    .await
    .unwrap();
    assert!(result.is_err());
}

#[tokio::test]
async fn incompatible_protocol_is_rejected_before_audio_pairing() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = ClientConfig::from_toml(&format!(
        "server_url='http://{}'",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let server = async {
        let (stream, _) = listener.accept().await.unwrap();
        let mut control = accept_async(stream).await.unwrap();
        control.next().await.unwrap().unwrap();
        let hello = ServerCommand::Hello {
            protocol_version: 9,
            session_id: "s1".into(),
            bridge_id: "b1".into(),
            generation: 7,
        };
        control
            .send(Message::text(serde_json::to_string(&hello).unwrap()))
            .await
            .unwrap();
    };
    let (_, result) = tokio::join!(
        server,
        run_once(&config, SimulatedBackend::new(config.max_buffer_samples))
    );
    assert!(result.unwrap_err().contains("protocol"));
}
