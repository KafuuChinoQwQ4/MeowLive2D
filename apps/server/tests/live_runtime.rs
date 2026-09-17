use axum::{
    Json, Router,
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
use meowlive_adapters::{
    live::bilibili::{BilibiliConfig, BilibiliLiveSource},
    llm::openai_compatible::{LlmConfig, OpenAiCompatible},
    speech::gpt_sovits::{GptSovits, GptSovitsConfig},
};
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use meowlive_protocol::{
    agent::{AgentEventStatus, AgentSnapshot, EventPayload},
    live::{LiveConnectionPhase, LiveConnectionSnapshot},
};
use meowlive_server::{
    agent::run_agent, config::AppConfig, state::AppState, transport::http::router,
    worker::run_worker,
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, task::JoinHandle};

#[derive(Clone, Debug, Default)]
struct Observed {
    start_bodies: Vec<Value>,
    end_bodies: Vec<Value>,
    auth_valid: bool,
    signed_start: bool,
    llm_prompts: Vec<Value>,
    tts_bodies: Vec<Value>,
}

#[derive(Clone)]
struct ExternalState {
    websocket_url: String,
    observed: Arc<Mutex<Observed>>,
}

struct ExternalFixture {
    base_url: String,
    observed: Arc<Mutex<Observed>>,
    task: JoinHandle<()>,
}

impl ExternalFixture {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let observed = Arc::new(Mutex::new(Observed::default()));
        let state = ExternalState {
            websocket_url: format!("ws://{address}/platform-ws"),
            observed: observed.clone(),
        };
        let app = Router::new()
            .route("/v2/app/start", post(platform_start))
            .route("/v2/app/heartbeat", post(platform_ok))
            .route("/v2/app/end", post(platform_end))
            .route("/platform-ws", get(platform_websocket))
            .route("/v1/chat/completions", post(llm_completion))
            .route("/tts", post(tts))
            .with_state(state);
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            base_url: format!("http://{address}"),
            observed,
            task,
        }
    }

    async fn observed(&self) -> Observed {
        self.observed.lock().await.clone()
    }
}

impl Drop for ExternalFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct ServerHarness {
    state: AppState,
    base_url: String,
    tasks: Vec<JoinHandle<()>>,
}

impl ServerHarness {
    async fn start(external: &ExternalFixture) -> Self {
        let mut config = AppConfig::default();
        config.agent.gift_merge_ms = 0;
        config.llm.base_url = format!("{}/v1", external.base_url);
        config.llm.model = "controlled-model".into();
        config.llm.api_key_env.clear();
        config.llm.timeout_seconds = 2;
        config.llm.max_retries = 0;
        config.speech.base_url = external.base_url.clone();
        config.speech.reference_audio = "/controlled/reference.wav".into();
        config.speech.prompt_text = "测试参考".into();
        config.speech.timeout_seconds = 2;
        config.live.enabled = true;
        config.live.app_id = 42;
        config.live.reconnect_initial_ms = 10;
        config.live.reconnect_max_ms = 20;
        config.validate().unwrap();

        let model = OpenAiCompatible::new(LlmConfig {
            base_url: config.llm.base_url.clone(),
            model: config.llm.model.clone(),
            api_key: Some("integration-key".into()),
            timeout: Duration::from_secs(config.llm.timeout_seconds),
            max_response_bytes: config.llm.max_response_bytes,
            max_tokens: config.llm.max_tokens,
            json_mode: config.llm.json_mode,
        })
        .unwrap();
        let synthesizer = GptSovits::new(GptSovitsConfig {
            base_url: config.speech.base_url.clone(),
            reference_audio: config.speech.reference_audio.clone(),
            prompt_text: config.speech.prompt_text.clone(),
            prompt_language: config.speech.prompt_language.clone(),
            text_language: config.speech.text_language.clone(),
            timeout: Duration::from_secs(config.speech.timeout_seconds),
            max_audio_bytes: config.speech.max_audio_bytes,
        })
        .unwrap();
        let source = BilibiliLiveSource::new(BilibiliConfig {
            app_id: 42,
            access_key_id: "test-key".into(),
            access_key_secret: "test-secret".into(),
            identity_code: "identity-code".into(),
            api_base_url: external.base_url.clone(),
            request_timeout: Duration::from_secs(2),
            connect_timeout: Duration::from_secs(2),
            receive_timeout: Duration::from_secs(5),
            ..BilibiliConfig::default()
        })
        .unwrap();
        let state = AppState::with_services(
            config,
            Arc::new(synthesizer),
            Some(Arc::new(model)),
            Some(Arc::new(source)),
        );
        state.resources.select_voice("default").unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let app = router(state.clone());
        let tasks = vec![
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            }),
            tokio::spawn(run_worker(state.clone())),
            tokio::spawn(run_agent(state.clone())),
        ];
        Self {
            state,
            base_url,
            tasks,
        }
    }
}

impl Drop for ServerHarness {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

#[tokio::test]
async fn bilibili_gift_reaches_one_completed_playout_and_disconnect_ends_the_project() {
    let external = ExternalFixture::start().await;
    let server = ServerHarness::start(&external).await;
    let client = reqwest::Client::new();
    let desktop_config = ClientConfig {
        server_url: server.base_url.clone(),
        ..ClientConfig::default()
    };
    let desktop_abort;
    let desktop = tokio::spawn(async move {
        run_once(
            &desktop_config,
            SimulatedBackend::new(desktop_config.max_buffer_samples),
        )
        .await
    });
    desktop_abort = desktop.abort_handle();
    let _desktop_cleanup = AbortOnDrop(desktop_abort);

    await_bridge(&client, &server.base_url).await;
    let response = client
        .post(format!("{}/api/live/connect", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    let live = await_live_counts(&client, &server.base_url).await;
    assert_eq!(live.phase, LiveConnectionPhase::Connected);
    assert_eq!(live.room_id.as_deref(), Some("99"));
    assert_eq!(live.accepted_events, 1);
    assert_eq!(live.duplicate_events, 1);
    assert_eq!(live.rejected_events, 0);

    let queued = agent(&client, &server.base_url).await;
    assert!(queued.paused);
    assert_eq!(queued.events.len(), 1);
    assert_eq!(queued.events[0].event.id, "bilibili:99:gift-1");
    assert_eq!(queued.events[0].event.source, "bilibili");
    assert_eq!(queued.events[0].event.viewer, "viewer");
    assert_eq!(
        queued.events[0].event.kind,
        EventPayload::Gift {
            name: "Cat".into(),
            count: 2,
        }
    );

    let response = client
        .post(format!("{}/api/agent/resume", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let completed = await_completed(&client, &server.base_url).await;
    assert_eq!(completed.events[0].status, AgentEventStatus::Completed);
    assert!(completed.events[0].speech_id.is_some());

    let observed = external.observed().await;
    assert_eq!(
        observed.start_bodies,
        [json!({"code":"identity-code","app_id":42})]
    );
    assert!(observed.signed_start);
    assert!(observed.auth_valid);
    assert_eq!(observed.llm_prompts.len(), 1);
    assert_eq!(
        observed.llm_prompts[0]["events"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        observed.llm_prompts[0]["events"][0]["id"],
        "bilibili:99:gift-1"
    );
    assert_eq!(
        observed.llm_prompts[0]["events"][0]["kind"],
        json!({"type":"gift","name":"Cat","count":2})
    );
    assert_eq!(observed.tts_bodies.len(), 1);
    assert_eq!(observed.tts_bodies[0]["text"], "谢谢 viewer 的两份 Cat");
    assert_eq!(
        observed.tts_bodies[0]["ref_audio_path"],
        "/controlled/reference.wav"
    );
    assert_eq!(observed.tts_bodies[0]["prompt_text"], "测试参考");
    assert_eq!(observed.tts_bodies[0]["prompt_lang"], "zh");
    assert_eq!(observed.tts_bodies[0]["text_lang"], "zh");

    let response = client
        .post(format!("{}/api/live/disconnect", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    await_disconnected(&client, &server.base_url).await;
    let observed = external.observed().await;
    assert_eq!(
        observed.end_bodies,
        [json!({"app_id":42,"game_id":"game-1"})]
    );
    assert!(agent(&client, &server.base_url).await.paused);

    server.state.shutdown().await;
    assert!(
        tokio::time::timeout(Duration::from_secs(2), desktop)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
}

struct AbortOnDrop(tokio::task::AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn platform_start(
    State(state): State<ExternalState>,
    headers: HeaderMap,
    body: Bytes,
) -> Json<Value> {
    let body: Value = serde_json::from_slice(&body).unwrap();
    let signed = [
        "authorization",
        "x-bili-accesskeyid",
        "x-bili-content-md5",
        "x-bili-signature-method",
        "x-bili-signature-nonce",
        "x-bili-signature-version",
        "x-bili-timestamp",
    ]
    .iter()
    .all(|name| headers.contains_key(*name));
    let mut observed = state.observed.lock().await;
    observed.start_bodies.push(body);
    observed.signed_start = signed;
    drop(observed);
    Json(json!({
        "code": 0,
        "message": "ok",
        "request_id": "integration-start",
        "data": {
            "game_info": {"game_id": "game-1"},
            "websocket_info": {
                "auth_body": "opaque-auth-body",
                "wss_link": [state.websocket_url],
            },
            "anchor_info": {
                "room_id": 99,
                "uname": "anchor",
                "uface": "https://i0.hdslb.com/face.jpg",
                "uid": 0,
                "open_id": "anchor-open-id",
                "union_id": "anchor-union-id",
            }
        }
    }))
}

async fn platform_ok() -> Json<Value> {
    Json(json!({"code":0,"message":"ok","request_id":"integration-heartbeat","data":{}}))
}

async fn platform_end(State(state): State<ExternalState>, body: Bytes) -> Json<Value> {
    state
        .observed
        .lock()
        .await
        .end_bodies
        .push(serde_json::from_slice(&body).unwrap());
    Json(json!({"code":0,"message":"ok","request_id":"integration-end","data":{}}))
}

async fn platform_websocket(
    State(state): State<ExternalState>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(move |socket| serve_platform_websocket(socket, state))
}

async fn serve_platform_websocket(mut socket: WebSocket, state: ExternalState) {
    let Some(Ok(Message::Binary(authority))) = socket.next().await else {
        return;
    };
    let auth_valid = authority.len() >= 16
        && u32::from_be_bytes(authority[8..12].try_into().unwrap()) == 7
        && &authority[16..] == b"opaque-auth-body";
    state.observed.lock().await.auth_valid = auth_valid;
    if socket
        .send(Message::Binary(packet(8, 1, b"{}").into()))
        .await
        .is_err()
    {
        return;
    }
    let gift = json!({
        "cmd": "LIVE_OPEN_PLATFORM_SEND_GIFT",
        "data": {
            "room_id": 99,
            "uid": 0,
            "open_id": "viewer1",
            "union_id": "viewer-union-id",
            "uname": "viewer",
            "uface": "https://i0.hdslb.com/viewer.jpg",
            "gift_id": 100,
            "gift_name": "Cat",
            "gift_num": 2,
            "price": 1000,
            "r_price": 1000,
            "paid": true,
            "fans_medal_level": 1,
            "fans_medal_name": "Meow",
            "fans_medal_wearing_status": true,
            "guard_level": 0,
            "timestamp": 1700000001,
            "msg_id": "gift-1",
            "anchor_info": {
                "uid": 0,
                "open_id": "anchor-open-id",
                "union_id": "anchor-union-id",
                "uname": "anchor",
                "uface": "https://i0.hdslb.com/anchor.jpg"
            },
            "gift_icon": "https://i0.hdslb.com/gift.png",
            "combo_gift": false,
            "combo_info": {
                "combo_base_num": 2,
                "combo_count": 1,
                "combo_id": "combo-1",
                "combo_timeout": 3
            },
            "blind_gift": {"blind_gift_id": 0, "status": false}
        }
    });
    let event = packet(5, 0, gift.to_string().as_bytes());
    let duplicates = [event.as_slice(), event.as_slice()].concat();
    if socket
        .send(Message::Binary(duplicates.into()))
        .await
        .is_err()
    {
        return;
    }
    while let Some(Ok(message)) = socket.next().await {
        match message {
            Message::Close(_) => break,
            Message::Ping(bytes) => {
                let _ = socket.send(Message::Pong(bytes)).await;
            }
            Message::Binary(bytes)
                if bytes.len() >= 12
                    && u32::from_be_bytes(bytes[8..12].try_into().unwrap()) == 2 =>
            {
                let _ = socket
                    .send(Message::Binary(packet(3, 1, &0_u32.to_be_bytes()).into()))
                    .await;
            }
            _ => {}
        }
    }
}

async fn llm_completion(
    State(state): State<ExternalState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    assert_eq!(headers["authorization"], "Bearer integration-key");
    assert_eq!(body["model"], "controlled-model");
    let prompt: Value =
        serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
    let event_id = prompt["events"][0]["id"].as_str().unwrap().to_owned();
    state.observed.lock().await.llm_prompts.push(prompt);
    Json(json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {
                "content": json!({
                    "reply_to": [event_id],
                    "text": "谢谢 viewer 的两份 Cat",
                    "topic": "礼物"
                }).to_string()
            }
        }]
    }))
}

async fn tts(
    State(state): State<ExternalState>,
    Json(body): Json<Value>,
) -> ([(&'static str, &'static str); 1], Vec<u8>) {
    state.observed.lock().await.tts_bodies.push(body);
    ([("content-type", "audio/wav")], wav())
}

fn packet(operation: u32, version: u16, body: &[u8]) -> Vec<u8> {
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

fn wav() -> Vec<u8> {
    let data_size = 4_800_u32;
    let mut bytes = Vec::with_capacity((44 + data_size) as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&24_000_u32.to_le_bytes());
    bytes.extend_from_slice(&48_000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for _ in 0..2_400 {
        bytes.extend_from_slice(&1_000_i16.to_le_bytes());
    }
    bytes
}

async fn await_bridge(client: &reqwest::Client, base_url: &str) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if agent(client, base_url).await.bridge_connected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

async fn await_live_counts(client: &reqwest::Client, base_url: &str) -> LiveConnectionSnapshot {
    let mut last = None;
    let result = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot: LiveConnectionSnapshot = client
                .get(format!("{base_url}/api/live"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if snapshot.accepted_events == 1 && snapshot.duplicate_events == 1 {
                return snapshot;
            }
            last = Some(snapshot);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    result.unwrap_or_else(|_| panic!("live counts did not settle; last snapshot: {last:?}"))
}

async fn await_completed(client: &reqwest::Client, base_url: &str) -> AgentSnapshot {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let snapshot = agent(client, base_url).await;
            if snapshot
                .events
                .first()
                .is_some_and(|event| event.status == AgentEventStatus::Completed)
            {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

async fn await_disconnected(client: &reqwest::Client, base_url: &str) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot: LiveConnectionSnapshot = client
                .get(format!("{base_url}/api/live"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if snapshot.phase == LiveConnectionPhase::Disconnected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

async fn agent(client: &reqwest::Client, base_url: &str) -> AgentSnapshot {
    client
        .get(format!("{base_url}/api/agent"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}
