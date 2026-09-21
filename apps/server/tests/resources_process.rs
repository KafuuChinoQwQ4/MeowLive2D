mod process_support;
use axum::{
    Json, Router,
    response::{IntoResponse, Response},
    routing::post,
};
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use process_support::ServerProcess;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Mutex;

struct Files(PathBuf);
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn reference_wav() -> Vec<u8> {
    let mut data = process_support::wav();
    data.resize(44, 0);
    let samples = 24_000u32 * 3;
    data[4..8].copy_from_slice(&(36 + samples * 2).to_le_bytes());
    data[40..44].copy_from_slice(&(samples * 2).to_le_bytes());
    for index in 0..samples {
        data.extend((if index % 2 == 0 { 2000i16 } else { -2000i16 }).to_le_bytes());
    }
    data
}
async fn get(client: &reqwest::Client, base: &str, path: &str) -> Value {
    client
        .get(format!("{base}{path}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}
async fn post_json(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    value: Value,
    expected: u16,
) -> Value {
    let response = client
        .post(format!("{base}{path}"))
        .json(&value)
        .send()
        .await
        .unwrap();
    let code = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    assert_eq!(code, expected, "{body}");
    body
}
async fn upload(
    client: &reqwest::Client,
    base: &str,
    audio: &[u8],
    metadata: Value,
    expected: u16,
) -> Value {
    let boundary = "meowlive-resource-test-boundary";
    let mut body=format!("--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"\r\n\r\n{metadata}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"reference.wav\"\r\nContent-Type: audio/wav\r\n\r\n").into_bytes();
    body.extend(audio);
    body.extend(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = client
        .post(format!("{base}/api/voices"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .unwrap();
    let code = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    assert_eq!(code, expected, "{body}");
    body
}
async fn await_speech(client: &reqwest::Client, base: &str, id: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let state = get(client, base, "/api/status").await;
            let task = state["speeches"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["id"] == id)
                .unwrap();
            assert_ne!(task["status"], "failed", "{task}");
            if task["status"] == "completed" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn uploaded_voice_survives_restart_and_reaches_tts_device_and_agent() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/resources-process")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root).unwrap();
    let _files = Files(root.clone());
    let calls = Arc::new(Mutex::new(Vec::<Value>::new()));
    let seen = calls.clone();
    let engine=Router::new().route("/tts",post(move|Json(body):Json<Value>|{let seen=seen.clone();async move{
        let path=body["ref_audio_path"].as_str().unwrap();
        assert!(path.starts_with('/'));assert!(std::fs::metadata(path).unwrap().len()>44);
        assert_eq!(body["prompt_text"],"这是上传音色的参考文本");
        assert_eq!(body["prompt_lang"],"zh");
        seen.lock().await.push(body);
        ([("content-type","audio/wav")],process_support::wav())
    }})).route("/v1/chat/completions",post(|Json(body):Json<Value>|async move{
        let dynamic_user=body["messages"].as_array().unwrap().iter().rev()
                .find(|message| message["role"]=="user").unwrap();
            let events:Value=serde_json::from_str(dynamic_user["content"].as_str().unwrap()).unwrap();
        completion_response(&body, json!({"reply_to":[events["events"][0]["id"]],"text":"这是当前音色的自动回复","topic":"欢迎"}))
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let engine = tokio::spawn(async {
        axum::serve(listener, engine).await.unwrap();
    });
    let config = format!(
        "[server]\nlisten_address='127.0.0.1:0'\n[speech]\nbase_url='{base}'\n[llm]\nbase_url='{base}/v1'\nmodel='controlled'\napi_key_env=''\n[resources]\ndirectory={}\n",
        serde_json::to_string(&root).unwrap()
    );
    let first = ServerProcess::start(&config).await;
    let client = reqwest::Client::new();
    let initial = get(&client, &first.base, "/api/resources").await;
    assert_eq!(initial["active_voice_id"], "");
    assert_eq!(initial["voices"], json!([]));
    assert_eq!(initial["default_voice_available"], false);
    let training = get(&client, &first.base, "/api/training").await;
    assert_eq!(training["jobs"], json!([]));
    assert_eq!(training["versions"], json!([]));
    let metadata =
        json!({"name":"上传音色","language":"zh","reference_text":"这是上传音色的参考文本"});
    upload(&client, &first.base, b"not-wave", metadata.clone(), 400).await;
    let snapshot = upload(&client, &first.base, &reference_wav(), metadata, 200).await;
    assert_eq!(snapshot["active_voice_id"], "");
    assert_eq!(snapshot["voices"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["voices"][0]["duration_ms"], 3000);
    assert_eq!(snapshot["voices"][0]["available"], true);
    assert!(!snapshot.to_string().contains("references/"));
    let id = snapshot["voices"][0]["id"].as_str().unwrap().to_owned();
    post_json(
        &client,
        &first.base,
        "/api/voices/select",
        json!({"id":id}),
        200,
    )
    .await;
    drop(first);
    let process = ServerProcess::start(&config).await;
    let snapshot = get(&client, &process.base, "/api/resources").await;
    assert_eq!(snapshot["active_voice_id"], id);
    let desktop_config = ClientConfig {
        server_url: process.base.clone(),
        ..Default::default()
    };
    let desktop = tokio::spawn(async move {
        run_once(
            &desktop_config,
            SimulatedBackend::new(desktop_config.max_buffer_samples),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(3), async {
        while get(&client, &process.base, "/api/status").await["bridge_connected"] != true {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let speech = post_json(
        &client,
        &process.base,
        "/api/speech",
        json!({"text":"上传音色试听","voice_id":id}),
        202,
    )
    .await;
    await_speech(&client, &process.base, speech["id"].as_str().unwrap()).await;
    post_json(&client, &process.base, "/api/agent/resume", json!({}), 200).await;
    post_json(&client,&process.base,"/api/events",json!({"events":[{"id":"voice-agent","source":"simulator","viewer":"观众","kind":{"type":"chat","text":"你好"}}]}),202).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let agent = get(&client, &process.base, "/api/agent").await;
            if agent["events"][0]["status"] == "completed" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let observed = calls.lock().await.clone();
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0]["ref_audio_path"], observed[1]["ref_audio_path"]);
    let speeches = get(&client, &process.base, "/api/status").await;
    assert!(
        speeches["speeches"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["voice_id"] == id)
    );
    std::fs::remove_file(observed[0]["ref_audio_path"].as_str().unwrap()).unwrap();
    let snapshot = get(&client, &process.base, "/api/resources").await;
    assert_eq!(snapshot["voices"][0]["available"], false);
    post_json(
        &client,
        &process.base,
        "/api/speech",
        json!({"text":"缺失后不能播报","voice_id":"active"}),
        400,
    )
    .await;
    assert_eq!(calls.lock().await.len(), 2);
    let deleted = post_json(
        &client,
        &process.base,
        "/api/voices/delete",
        json!({"id":id}),
        200,
    )
    .await;
    assert_eq!(deleted["voices"], json!([]));
    assert_eq!(deleted["active_voice_id"], "");
    drop(process);
    assert!(
        tokio::time::timeout(Duration::from_secs(2), desktop)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    engine.abort();
    let reopened = ServerProcess::start(&config).await;
    let snapshot = get(&client, &reopened.base, "/api/resources").await;
    assert_eq!(snapshot["voices"], json!([]));
    assert_eq!(snapshot["active_voice_id"], "");
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
