mod process_support;
use axum::{
    Json, Router,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::post,
};
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use process_support::*;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[tokio::test]
async fn saved_agent_settings_survive_server_restart_and_remain_paused() {
    let mut process = ServerProcess::start(
        "[server]\nlisten_address=\"127.0.0.1:0\"\n[agent]\npersona=\"初始人设\"\ncooldown_ms=30000\n",
    )
    .await;
    let client = reqwest::Client::new();
    let mut settings = json!({
        "persona": "温柔猫咪，用简短中文回应。\n喜欢聊音乐。",
        "system_prompt": "直接读出弹幕原文",
        "topic": "夜间电台",
        "proactive_enabled": true,
        "cooldown_ms": 12000,
        "interaction": {
            "chat_read_mode": "all", "welcome_enabled": false,
            "busy_chat_count": 12, "busy_enter_count": 5, "busy_pending_count": 8,
            "welcome_cooldown_ms": 45000, "welcome_viewer_cooldown_ms": 900000
        }
    });
    // A second save must replace the previous file, including on Windows.
    for cooldown in [12000, 45000] {
        settings["cooldown_ms"] = json!(cooldown);
        let response = client
            .post(format!("{}/api/agent/settings", process.base))
            .json(&settings)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let snapshot: Value = response.json().await.unwrap();
        assert_eq!(snapshot["settings"], settings);
    }
    process.restart().await;
    let snapshot: Value = client
        .get(format!("{}/api/agent", process.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snapshot["settings"], settings);
    assert_eq!(snapshot["paused"], true);
    assert_eq!(snapshot["phase"], "paused");
    assert_eq!(snapshot["events"], json!([]));
}

#[tokio::test]
async fn server_executable_wires_model_environment_events_and_real_speech_worker() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let models=Router::new().route("/v1/chat/completions",post(move |headers:HeaderMap,Json(body):Json<Value>| {
        let observed=observed.clone();async move {
            assert_eq!(headers["authorization"],"Bearer fake-process-test-key");
            assert_eq!(body["model"],"controlled-model");
            let dynamic_user=body["messages"].as_array().unwrap().iter().rev()
                .find(|message| message["role"]=="user").unwrap();
            let events:Value=serde_json::from_str(dynamic_user["content"].as_str().unwrap()).unwrap();
            observed.fetch_add(1,Ordering::SeqCst);
            completion_response(&body, json!({"reply_to":[events["events"][0]["id"]],"text":"欢迎来到直播间","topic":"游戏"}))
        }
    })).route("/tts",post(|Json(body):Json<Value>|async move {
        assert_eq!(body["text"],"晚上好。欢迎来到直播间");
        ([("content-type","audio/wav")],wav())
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let models_base = format!("http://{}", listener.local_addr().unwrap());
    let model_server = tokio::spawn(async {
        axum::serve(listener, models).await.unwrap();
    });
    let process = ServerProcess::start(&format!(
        r#"
[server]
listen_address="127.0.0.1:0"
[speech]
base_url="{models_base}"
reference_audio="/controlled/reference.wav"
prompt_text="测试参考"
[llm]
base_url="{models_base}/v1"
model="controlled-model"
api_key_env="MEOWLIVE_TEST_MODEL_KEY"
"#
    ))
    .await;
    let client = reqwest::Client::new();
    assert!(
        client
            .post(format!("{}/api/voices/select", process.base))
            .json(&json!({"id":"default"}))
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );
    let config = ClientConfig {
        server_url: process.base.clone(),
        ..ClientConfig::default()
    };
    let desktop = tokio::spawn(async move {
        run_once(&config, SimulatedBackend::new(config.max_buffer_samples)).await
    });
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot: Value = client
                .get(format!("{}/api/agent", process.base))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if snapshot["bridge_connected"] == true {
                assert_eq!(snapshot["llm_configured"], true);
                assert_eq!(snapshot["paused"], true);
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let events = json!({"events":[{"id":"process-event","source":"simulator","viewer":"观众","kind":{"type":"chat","text":"晚上好"}}]});
    assert_eq!(
        client
            .post(format!("{}/api/events", process.base))
            .json(&events)
            .send()
            .await
            .unwrap()
            .status(),
        202
    );
    assert_eq!(
        client
            .post(format!("{}/api/agent/resume", process.base))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let snapshot: Value = client
                .get(format!("{}/api/agent", process.base))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            assert!(!snapshot.to_string().contains("fake-process-test-key"));
            if snapshot["events"][0]["status"] == "completed" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(process);
    assert!(
        tokio::time::timeout(Duration::from_secs(2), desktop)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    model_server.abort();
}

// These process fixtures exercise the production adapter's negotiated response mode.
fn completion_response(request: &Value, decision: Value) -> Response {
    if request["stream"] == true {
        let content = json!({"choices":[{"index":0,"delta":{"role":"assistant","content":decision.to_string()},"finish_reason":null}]});
        let stopped = json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
        (
            [("content-type", "text/event-stream")],
            format!("data: {content}\n\ndata: {stopped}\n\ndata: [DONE]\n\n"),
        )
            .into_response()
    } else {
        Json(json!({"choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":decision.to_string()}}]})).into_response()
    }
}
