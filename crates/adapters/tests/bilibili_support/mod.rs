#![allow(dead_code)]

use std::io::Write;

use axum::{
    Router,
    body::Bytes,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::HeaderMap,
    response::Response,
    routing::{get, post},
};
use futures_util::StreamExt;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn packet(operation: u32, version: u16, body: &[u8]) -> Vec<u8> {
    let length = 16 + body.len();
    let mut bytes = Vec::with_capacity(length);
    bytes.extend_from_slice(&(length as u32).to_be_bytes());
    bytes.extend_from_slice(&16_u16.to_be_bytes());
    bytes.extend_from_slice(&version.to_be_bytes());
    bytes.extend_from_slice(&operation.to_be_bytes());
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.extend_from_slice(body);
    bytes
}

pub fn notify_packet(body: &[u8]) -> Vec<u8> {
    packet(5, 0, body)
}

pub fn zlib_packet(inner: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(inner).unwrap();
    packet(5, 2, &encoder.finish().unwrap())
}

pub fn brotli_packet(inner: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 5, 20);
        writer.write_all(inner).unwrap();
    }
    packet(5, 3, &compressed)
}

#[derive(Clone, Copy)]
pub enum FixtureMode {
    Events,
    EmptyAuth,
    InteractionEnd,
    AlternateWebsocket,
    IncompleteStart,
    MalformedStart,
    RejectAuth,
    Silent,
    SlowStart,
}

#[derive(Clone, Debug, Default)]
pub struct Observed {
    pub start_bodies: Vec<String>,
    pub end_bodies: Vec<String>,
    pub auth_bodies: Vec<String>,
    pub heartbeat_bodies: Vec<String>,
    pub start_signature_valid: bool,
}

#[derive(Clone)]
struct FixtureState {
    mode: FixtureMode,
    ws_url: String,
    observed: Arc<Mutex<Observed>>,
}

pub struct PlatformFixture {
    pub base_url: String,
    observed: Arc<Mutex<Observed>>,
}

impl PlatformFixture {
    pub async fn start(mode: FixtureMode) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let observed = Arc::new(Mutex::new(Observed::default()));
        let state = FixtureState {
            mode,
            ws_url: format!("ws://{address}/ws"),
            observed: observed.clone(),
        };
        let router = Router::new()
            .route("/v2/app/start", post(start_handler))
            .route("/v2/app/heartbeat", post(heartbeat_handler))
            .route("/v2/app/end", post(end_handler))
            .route("/ws", get(ws_handler))
            .with_state(state);
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self {
            base_url: format!("http://{address}"),
            observed,
        }
    }

    pub async fn observed(&self) -> Observed {
        self.observed.lock().await.clone()
    }
}

async fn start_handler(
    State(state): State<FixtureState>,
    headers: HeaderMap,
    body: Bytes,
) -> axum::Json<serde_json::Value> {
    if matches!(state.mode, FixtureMode::SlowStart) {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let signature_valid = verify_signature(&headers, &body);
    let mut observed = state.observed.lock().await;
    observed.start_bodies.push(body_text);
    observed.start_signature_valid = signature_valid;
    drop(observed);
    if matches!(state.mode, FixtureMode::MalformedStart) {
        return axum::Json(json!({ "code": 0, "message": "ok", "data": "invalid" }));
    }
    let websocket_urls = if matches!(state.mode, FixtureMode::AlternateWebsocket) {
        vec!["ws://127.0.0.1:1".to_string(), state.ws_url]
    } else if matches!(state.mode, FixtureMode::IncompleteStart) {
        Vec::new()
    } else {
        vec![state.ws_url]
    };
    axum::Json(json!({
        "code": 0,
        "message": "ok",
        "data": {
            "game_info": { "game_id": "game-1" },
            "websocket_info": { "auth_body": "auth-body", "wss_link": websocket_urls },
            "anchor_info": { "room_id": 99 }
        }
    }))
}

async fn ok_handler() -> axum::Json<serde_json::Value> {
    axum::Json(json!({ "code": 0, "message": "ok", "data": {} }))
}

async fn heartbeat_handler(
    State(state): State<FixtureState>,
    body: Bytes,
) -> axum::Json<serde_json::Value> {
    state
        .observed
        .lock()
        .await
        .heartbeat_bodies
        .push(String::from_utf8(body.to_vec()).unwrap());
    ok_handler().await
}

async fn end_handler(
    State(state): State<FixtureState>,
    body: Bytes,
) -> axum::Json<serde_json::Value> {
    state
        .observed
        .lock()
        .await
        .end_bodies
        .push(String::from_utf8(body.to_vec()).unwrap());
    ok_handler().await
}

async fn ws_handler(State(state): State<FixtureState>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: FixtureState) {
    let Some(Ok(Message::Binary(auth))) = socket.next().await else {
        return;
    };
    if auth.len() >= 16 {
        state
            .observed
            .lock()
            .await
            .auth_bodies
            .push(String::from_utf8_lossy(&auth[16..]).into());
    }
    let code = if matches!(state.mode, FixtureMode::RejectAuth) {
        -1
    } else {
        0
    };
    let response = if matches!(state.mode, FixtureMode::EmptyAuth) {
        packet(8, 1, &[])
    } else {
        packet(8, 1, json!({ "code": code }).to_string().as_bytes())
    };
    if socket.send(Message::Binary(response.into())).await.is_err() || code != 0 {
        return;
    }
    if matches!(state.mode, FixtureMode::Events) {
        let messages = [
            notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":99,"msg_id":"dm-1","uname":"Alice","timestamp":1700000000,"msg":"hello"}}"#),
            notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_SEND_GIFT","data":{"room_id":99,"msg_id":"gift-1","uname":"Bob","timestamp":1700000001,"gift_name":"Cat","gift_num":2}}"#),
        ].concat();
        let _ = socket.send(Message::Binary(messages.into())).await;
    } else if matches!(state.mode, FixtureMode::InteractionEnd) {
        let ended = notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_INTERACTION_END","data":{"game_id":"game-1","timestamp":1700000000}}"#);
        let _ = socket.send(Message::Binary(ended.into())).await;
    }
    while let Some(Ok(message)) = socket.next().await {
        match message {
            Message::Close(_) => break,
            Message::Ping(bytes) => {
                let _ = socket.send(Message::Pong(bytes)).await;
            }
            _ => {}
        }
    }
}

fn verify_signature(headers: &HeaderMap, body: &[u8]) -> bool {
    let get = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let (Some(timestamp), Some(nonce), Some(authorization)) = (
        get("x-bili-timestamp"),
        get("x-bili-signature-nonce"),
        get("authorization"),
    ) else {
        return false;
    };
    meowlive_adapters::live::bilibili::signing::sign(
        "test-key",
        "test-secret",
        body,
        timestamp.parse().unwrap_or_default(),
        nonce,
    )
    .is_ok_and(|signed| signed.authorization == authorization)
}
