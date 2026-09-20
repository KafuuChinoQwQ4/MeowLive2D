#![allow(dead_code)]
use axum::{Router, body::Body, http::Request};
use http_body_util::BodyExt;
use meowlive_application::ports::speech::{
    PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest,
};
use meowlive_server::{config::AppConfig, state::AppState, transport::http::router};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

struct FixedSpeech;
impl SpeechSynthesizer for FixedSpeech {
    fn synthesize(&self, _request: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async {
            Ok(PcmAudio {
                sample_rate: 32000,
                channels: 1,
                samples: vec![1, -1, 0, 0],
            })
        })
    }
}

pub fn state() -> AppState {
    let mut config = AppConfig::default();
    config.viewers.enabled = false;
    AppState::new(config, Arc::new(FixedSpeech))
}
pub fn state_with_config(config: AppConfig) -> AppState {
    AppState::new(config, Arc::new(FixedSpeech))
}
pub async fn request(app: Router, method: &str, path: &str, body: Value) -> (u16, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn status(state: &AppState) -> Value {
    request(router(state.clone()), "GET", "/api/status", json!({}))
        .await
        .1
}
pub async fn server() -> (AppState, String, tokio::task::JoinHandle<()>) {
    let state = state();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("ws://{}", listener.local_addr().unwrap());
    let app = router(state.clone());
    let task = tokio::spawn(async {
        axum::serve(listener, app).await.unwrap();
    });
    (state, base, task)
}

pub type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

pub async fn next_json(socket: &mut Socket) -> Value {
    let frame = next_data(socket).await;
    serde_json::from_str(frame.to_text().unwrap()).unwrap()
}

pub async fn next_data(socket: &mut Socket) -> tokio_tungstenite::tungstenite::Message {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            match socket.next().await.unwrap().unwrap() {
                Message::Ping(_) | Message::Pong(_) => socket.flush().await.unwrap(),
                data => return data,
            }
        }
    })
    .await
    .unwrap()
}

pub async fn pair(base: &str) -> (Socket, Socket, Value) {
    use futures_util::SinkExt;
    let (mut control, _) = tokio_tungstenite::connect_async(format!("{base}/ws/control"))
        .await
        .unwrap();
    control
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({"type":"hello","protocol_version":meowlive_protocol::PROTOCOL_VERSION})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let hello = next_json(&mut control).await;
    let url = format!(
        "{base}/ws/audio?session_id={}&bridge_id={}",
        hello["session_id"].as_str().unwrap(),
        hello["bridge_id"].as_str().unwrap()
    );
    let (audio, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    (control, audio, hello)
}

pub async fn await_connected(state: &AppState, expected: bool) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if status(state).await["bridge_connected"] == expected {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
