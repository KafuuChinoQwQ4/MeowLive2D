#![allow(dead_code)]

use futures_util::{SinkExt, StreamExt};
use meowlive_desktop_runtime::avatar::{self, AvatarStatus, VtsConfig};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
    task::JoinHandle,
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

pub type Socket = WebSocketStream<TcpStream>;
static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    pub config: VtsConfig,
    pub listener: TcpListener,
    root: PathBuf,
}
impl Fixture {
    pub async fn new(cached: bool) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/avatar-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&root).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = VtsConfig {
            enabled: true,
            websocket_url: format!("ws://{}", listener.local_addr().unwrap()),
            token_path: root.join("token.json"),
            request_timeout_ms: 150,
            auth_timeout_ms: 500,
            reconnect_delay_ms: 50,
            ..VtsConfig::default()
        };
        if cached {
            write_token(&config.token_path, "cached-private-token");
        }
        Self {
            config,
            listener,
            root,
        }
    }
    pub fn start(
        &self,
    ) -> (
        watch::Sender<f64>,
        watch::Receiver<AvatarStatus>,
        JoinHandle<()>,
    ) {
        let (levels, receiver) = watch::channel(0.0);
        let (status, statuses) = watch::channel(AvatarStatus::Disabled);
        let task = tokio::spawn(avatar::run(self.config.clone(), receiver, status));
        (levels, statuses, task)
    }
    pub async fn accept(&self) -> Socket {
        let (stream, _) = tokio::time::timeout(Duration::from_secs(2), self.listener.accept())
            .await
            .unwrap()
            .unwrap();
        tokio_tungstenite::accept_async(stream).await.unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
pub fn write_token(path: &std::path::Path, token: &str) {
    std::fs::write(path, json!({"authenticationToken":token}).to_string()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
}
pub async fn request(socket: &mut Socket, kind: &str) -> Value {
    let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
    assert_eq!(value["apiName"], "VTubeStudioPublicAPI");
    assert_eq!(value["apiVersion"], "1.0");
    assert_eq!(value["messageType"], kind);
    let id = value["requestID"].as_str().unwrap();
    assert!(!id.is_empty() && id.len() <= 64 && id.is_ascii());
    value
}
pub async fn respond(socket: &mut Socket, request: &Value, kind: &str, data: Value) {
    socket
        .send(Message::Text(
            json!({"apiName":"VTubeStudioPublicAPI","apiVersion":"1.0",
        "requestID":request["requestID"],"messageType":kind,"data":data})
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
}
pub async fn authenticate(socket: &mut Socket) {
    let req = request(socket, "AuthenticationRequest").await;
    assert_eq!(req["data"]["pluginName"], "MeowLive2D");
    assert_eq!(req["data"]["pluginDeveloper"], "MeowLive2D");
    assert_eq!(req["data"]["authenticationToken"], "cached-private-token");
    respond(
        socket,
        &req,
        "AuthenticationResponse",
        json!({"authenticated":true}),
    )
    .await;
    let req = request(socket, "ParameterCreationRequest").await;
    assert_eq!(req["data"]["parameterName"], "MeowMouthOpen");
    assert_eq!(req["data"]["min"], 0.0);
    assert_eq!(req["data"]["max"], 1.0);
    assert_eq!(req["data"]["defaultValue"], 0.0);
    respond(
        socket,
        &req,
        "ParameterCreationResponse",
        json!({"parameterName":"MeowMouthOpen"}),
    )
    .await;
}
pub async fn injected(socket: &mut Socket) -> f64 {
    let req = request(socket, "InjectParameterDataRequest").await;
    assert_eq!(req["data"]["mode"], "set");
    assert!(req["data"].get("faceFound").is_none());
    assert_eq!(req["data"]["parameterValues"].as_array().unwrap().len(), 1);
    assert_eq!(req["data"]["parameterValues"][0]["id"], "MeowMouthOpen");
    let value = req["data"]["parameterValues"][0]["value"].as_f64().unwrap();
    respond(socket, &req, "InjectParameterDataResponse", json!({})).await;
    value
}
pub async fn finished(task: JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("worker must stop")
        .unwrap();
}
pub async fn status_is(status: &mut watch::Receiver<AvatarStatus>, expected: AvatarStatus) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if *status.borrow_and_update() == expected {
                return;
            }
            status.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
}
