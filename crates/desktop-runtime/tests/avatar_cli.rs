mod avatar_support;
use avatar_support::*;
use futures_util::{SinkExt, StreamExt};
use meowlive_protocol::{
    audio::{AudioChunk, AudioFormat},
    control::ServerCommand,
};
use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio::{net::TcpListener, sync::oneshot};
use tokio_tungstenite::{accept_async, tungstenite::Message};

struct ClientProcess(Child);
impl Drop for ClientProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn executable_connects_vts_and_resets_mouth_when_bridge_disconnects() {
    let fixture = Fixture::new(true).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config_path = fixture.config.token_path.with_file_name("client.toml");
    std::fs::write(&config_path, format!("server_url='http://{}'\n[vtube_studio]\nenabled=true\nwebsocket_url='{}'\ntoken_path='token.json'\n",listener.local_addr().unwrap(),fixture.config.websocket_url)).unwrap();
    let (disconnect, disconnected) = oneshot::channel();
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
        for sequence in 0..2 {
            let chunk = AudioChunk {
                utterance_id: "u1".into(),
                generation: 1,
                sequence,
                samples: vec![16000; 12000],
                end: sequence == 1,
            };
            audio
                .send(Message::binary(chunk.encode().unwrap()))
                .await
                .unwrap();
        }
        disconnected.await.unwrap();
        control.close(None).await.unwrap();
    });
    let mut client = ClientProcess(
        Command::new(env!("CARGO_BIN_EXE_meowlive-client"))
            .args(["--simulate", "--once", "--config"])
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    tokio::time::timeout(Duration::from_secs(2), async {
        while injected(&mut socket).await <= 0.0 {}
    })
    .await
    .unwrap();
    disconnect.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while injected(&mut socket).await != 0.0 {}
    })
    .await
    .unwrap();
    server.await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while client.0.try_wait().unwrap().is_none() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
