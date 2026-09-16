use axum::{
    Json, Router,
    extract::{WebSocketUpgrade, ws::Message},
    http::StatusCode,
    routing::{get, post},
};
use meowlive_adapters::live::bilibili::{BilibiliConfig, BilibiliLiveSource};
use meowlive_application::ports::live_source::LiveSource;
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{sync::Notify, task::JoinHandle};

#[derive(Clone, Copy)]
enum Mode {
    CleanupFails,
    Incomplete,
    CoalescedAuth,
    SlowHeartbeat,
}
struct Fixture {
    config: BilibiliConfig,
    task: JoinHandle<()>,
    ends: Arc<AtomicUsize>,
    heartbeat_started: Arc<Notify>,
    heartbeat_release: Arc<Notify>,
    event_ready: Arc<Notify>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Fixture {
    async fn start(mode: Mode) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let ends = Arc::new(AtomicUsize::new(0));
        let end_count = ends.clone();
        let heartbeat_started = Arc::new(Notify::new());
        let heartbeat_release = Arc::new(Notify::new());
        let event_ready = Arc::new(Notify::new());
        let (started, release, event) = (
            heartbeat_started.clone(),
            heartbeat_release.clone(),
            event_ready.clone(),
        );
        let app = Router::new()
            .route("/v2/app/start", post(move || async move {
                let mut data = json!({"game_info":{"game_id":"game-test"},"websocket_info":{"auth_body":"opaque-auth","wss_link":[format!("ws://{address}/ws")]},"anchor_info":{"room_id":99}});
                if matches!(mode, Mode::CleanupFails) { data["websocket_info"]["wss_link"] = json!(["ws://127.0.0.1:1"]); }
                if matches!(mode, Mode::Incomplete) { data.as_object_mut().unwrap().remove("websocket_info"); }
                Json(json!({"code":0,"data":data}))
            }))
            .route("/v2/app/end", post(move || { let count = end_count.clone(); async move {
                count.fetch_add(1, Ordering::SeqCst);
                (if matches!(mode, Mode::CleanupFails) { StatusCode::INTERNAL_SERVER_ERROR } else { StatusCode::OK }, Json(json!({"code":0,"data":{}})))
            }}))
            .route("/v2/app/heartbeat", post(move || { let (started, release) = (started.clone(), release.clone()); async move {
                started.notify_one();
                if matches!(mode, Mode::SlowHeartbeat) { release.notified().await; }
                Json(json!({"code":0,"data":{}}))
            }}))
            .route("/ws", get(move |upgrade: WebSocketUpgrade| { let ready=event.clone(); async move {
                upgrade.on_upgrade(move |mut socket| async move {
                    if socket.recv().await.is_none() { return; }
                    let auth = packet(8, b"{}");
                    let gift = packet(5, br#"{"cmd":"LIVE_OPEN_PLATFORM_SEND_GIFT","data":{"room_id":99,"msg_id":"gift-1","uname":"viewer","timestamp":1780000000,"gift_name":"Cat","gift_num":2}}"#);
                    if matches!(mode, Mode::CoalescedAuth) {
                        let _ = socket.send(Message::Binary([auth, gift].concat().into())).await;
                    } else {
                        let _ = socket.send(Message::Binary(auth.into())).await;
                        ready.notified().await;
                        let _ = socket.send(Message::Binary(gift.into())).await;
                    }
                    while let Some(Ok(message)) = socket.recv().await {
                        if matches!(message, Message::Close(_)) { break; }
                    }
                })
            }}));
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let config = BilibiliConfig {
            app_id: 42,
            access_key_id: "test".into(),
            access_key_secret: "test-secret".into(),
            identity_code: "test-code".into(),
            api_base_url: format!("http://{address}"),
            request_timeout: Duration::from_secs(2),
            connect_timeout: Duration::from_secs(2),
            app_heartbeat_interval: Duration::from_secs(1),
            websocket_heartbeat_interval: Duration::from_secs(1),
            receive_timeout: Duration::from_secs(5),
            ..BilibiliConfig::default()
        };
        Self {
            config,
            task,
            ends,
            heartbeat_started,
            heartbeat_release,
            event_ready,
        }
    }
}
fn packet(op: u32, body: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend(((body.len() + 16) as u32).to_be_bytes());
    frame.extend(16u16.to_be_bytes());
    frame.extend(1u16.to_be_bytes());
    frame.extend(op.to_be_bytes());
    frame.extend(1u32.to_be_bytes());
    frame.extend(body);
    frame
}

#[tokio::test]
async fn failed_cleanup_after_websocket_connect_failure_must_not_create_another_session() {
    let fixture = Fixture::start(Mode::CleanupFails).await;
    let source = BilibiliLiveSource::new(fixture.config.clone()).unwrap();
    let error = source.connect().await.err().expect("connection must fail");
    assert_eq!(fixture.ends.load(Ordering::SeqCst), 1);
    assert!(
        !error.retryable,
        "failed project cleanup must stop automatic creation of new projects"
    );
}

#[tokio::test]
async fn incomplete_start_with_known_game_id_still_ends_the_project() {
    let fixture = Fixture::start(Mode::Incomplete).await;
    let source = BilibiliLiveSource::new(fixture.config.clone()).unwrap();
    assert!(source.connect().await.is_err());
    assert_eq!(fixture.ends.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn notification_in_the_authentication_frame_is_delivered_once() {
    let fixture = Fixture::start(Mode::CoalescedAuth).await;
    let source = BilibiliLiveSource::new(fixture.config.clone()).unwrap();
    let mut connection = source.connect().await.unwrap();
    let event = connection.next().await.unwrap().unwrap();
    assert_eq!(event.id, "bilibili:99:gift-1");
    connection.close().await.unwrap();
}

#[tokio::test]
async fn slow_http_heartbeat_does_not_block_websocket_event_delivery() {
    let fixture = Fixture::start(Mode::SlowHeartbeat).await;
    let source = BilibiliLiveSource::new(fixture.config.clone()).unwrap();
    let mut connection = source.connect().await.unwrap();
    let mut next = connection.next();
    tokio::select! {
        _=fixture.heartbeat_started.notified()=>{},
        result=&mut next=>panic!("unexpected early result {result:?}"),
        _=tokio::time::sleep(Duration::from_secs(3))=>panic!("heartbeat was not started"),
    }
    fixture.event_ready.notify_one();
    let result = tokio::time::timeout(Duration::from_millis(300), &mut next).await;
    fixture.heartbeat_release.notify_one();
    assert!(result.is_ok(), "HTTP heartbeat blocked live events");
    assert_eq!(result.unwrap().unwrap().unwrap().id, "bilibili:99:gift-1");
    drop(next);
    connection.close().await.unwrap();
}

#[test]
fn empty_platform_message_identity_is_not_replaced_by_the_prefix() {
    use meowlive_adapters::live::bilibili::protocol::{DecodeLimits, decode_events};
    let frame=packet(5,br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":99,"msg_id":"","uname":"viewer","timestamp":1780000000,"msg":"hello"}}"#);
    assert!(
        decode_events(&frame, "99", DecodeLimits::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn decompression_budget_applies_to_the_whole_websocket_frame() {
    use meowlive_adapters::live::bilibili::protocol::{DecodeLimits, decode_events};
    use std::io::Write;
    let inner = packet(3, &[0; 20]);
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&inner).unwrap();
    let mut compressed = packet(5, &encoder.finish().unwrap());
    compressed[6..8].copy_from_slice(&2u16.to_be_bytes());
    let frame = [compressed.clone(), compressed].concat();
    let limits = DecodeLimits {
        max_decompressed_bytes: inner.len() + 1,
        ..DecodeLimits::default()
    };
    assert!(decode_events(&frame, "99", limits).is_err());
}
