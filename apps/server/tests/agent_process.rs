mod process_support;
use axum::{Json, Router, http::HeaderMap, routing::post};
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
async fn server_executable_wires_model_environment_events_and_real_speech_worker() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let models=Router::new().route("/v1/chat/completions",post(move |headers:HeaderMap,Json(body):Json<Value>| {
        let observed=observed.clone();async move {
            assert_eq!(headers["authorization"],"Bearer fake-process-test-key");
            assert_eq!(body["model"],"controlled-model");
            let events:Value=serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
            observed.fetch_add(1,Ordering::SeqCst);
            Json(json!({"choices":[{"finish_reason":"stop","message":{"content":json!({"reply_to":[events["events"][0]["id"]],"text":"欢迎来到直播间","topic":"游戏"}).to_string()}}]}))
        }
    })).route("/tts",post(|Json(body):Json<Value>|async move {
        assert_eq!(body["text"],"欢迎来到直播间");
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
