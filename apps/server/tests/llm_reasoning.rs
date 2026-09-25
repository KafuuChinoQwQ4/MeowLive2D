mod support;

use axum::{Json, Router, body::Body, http::Request, routing::post};
use meowlive_protocol::llm::LlmSettingsRequest;
use meowlive_server::{
    config::LlmConfig,
    llm_settings::{LlmSettingsStore, load_override, settings_path},
    transport::http::router,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

async fn send(app: Router, path: &str, body: Value) -> (u16, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = axum::body::to_bytes(response.into_body(), 65536)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

fn preview(effort: &str) -> Value {
    json!({"provider":"anthropic", "api_format":"anthropic_messages", "model":"claude-opus-4-6", "reasoning_effort":effort, "max_tokens":4096})
}

fn settings(base: &str, effort: Option<&str>) -> Value {
    let mut settings = json!({"provider":"openai", "api_format":"openai_chat", "base_url":base, "model":"gpt-5", "mode":"cloud", "timeout_seconds":10, "max_tokens":4096, "json_mode":true});
    if let Some(effort) = effort {
        settings["reasoning_effort"] = json!(effort);
    }
    json!({"settings":settings,"api_key":null,"clear_api_key":false})
}

#[tokio::test]
async fn preview_clamps_both_bounds_floors_gaps_and_preserves_default() {
    for (requested, effective) in [
        ("ultra", "max"),
        ("minimal", "low"),
        ("xhigh", "high"),
        ("medium", "medium"),
        ("default", "default"),
    ] {
        let (status, body) = send(
            router(support::state()),
            "/api/llm/reasoning",
            preview(requested),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["requested"], requested);
        assert_eq!(body["effective"], effective);
        assert_eq!(body["supported"], json!(["low", "medium", "high", "max"]));
        assert!(body["error"].is_null(), "{body}");
    }
}

#[tokio::test]
async fn preview_rejects_invalid_contracts_and_marks_unknown_models() {
    for change in [
        json!({"reasoning_effort":"super"}),
        json!({"max_tokens":65537}),
        json!({"api_key":"should-not-be-accepted"}),
        json!({"api_format":"unknown"}),
    ] {
        let mut body = preview("high");
        body.as_object_mut()
            .unwrap()
            .extend(change.as_object().unwrap().clone());
        let (status, _) = send(router(support::state()), "/api/llm/reasoning", body).await;
        assert_eq!(status, 400);
    }
    let mut body = preview("ultra");
    body["model"] = json!("unknown-future-model");
    let (status, result) = send(router(support::state()), "/api/llm/reasoning", body).await;
    assert_eq!(status, 200);
    assert!(result["effective"].is_null());
    assert_eq!(result["supported"], json!([]));
    assert!(!result["note"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn budget_errors_are_previewed_and_rejected_before_any_provider_call() {
    let mut body = preview("high");
    body["model"] = json!("claude-sonnet-4-5");
    body["max_tokens"] = json!(1024);
    let (status, result) = send(router(support::state()), "/api/llm/reasoning", body).await;
    assert_eq!(status, 200, "{result}");
    assert!(
        result["error"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "{result}"
    );
    let mut request = settings("http://127.0.0.1:1/v1", Some("high"));
    request["settings"]["provider"] = json!("anthropic");
    request["settings"]["api_format"] = json!("anthropic_messages");
    request["settings"]["model"] = json!("claude-sonnet-4-5");
    request["settings"]["max_tokens"] = json!(1024);
    let (status, result) = send(router(support::state()), "/api/llm/test", request).await;
    assert_eq!(status, 400, "{result}");
}

#[tokio::test]
async fn preview_requires_the_same_admin_authentication_as_settings() {
    let mut config = meowlive_server::config::AppConfig::default();
    config.auth.enabled = true;
    let (status, _) = send(
        router(support::state_with_config(config)),
        "/api/llm/reasoning",
        preview("high"),
    )
    .await;
    assert!(status == 401 || status == 503);
}

#[tokio::test]
async fn saves_requested_level_reloads_it_and_accepts_old_configuration() {
    let directory = std::env::temp_dir().join(format!("meow-reasoning-{}", uuid::Uuid::new_v4()));
    let config_path = directory.join("server.toml");
    let store = LlmSettingsStore::new(LlmConfig::default(), Some(settings_path(&config_path)));
    for effort in [None, Some("ultra"), Some("minimal"), Some("default")] {
        let request: LlmSettingsRequest =
            serde_json::from_value(settings("https://api.openai.com/v1", effort)).unwrap();
        let snapshot = store.save(request).await.unwrap();
        let snapshot = serde_json::to_value(snapshot).unwrap();
        assert_eq!(
            snapshot["settings"]["reasoning_effort"],
            effort.unwrap_or("default")
        );
        let active = load_override(&config_path).unwrap().unwrap();
        let restarted = LlmSettingsStore::new(active, Some(settings_path(&config_path)));
        let loaded = serde_json::to_value(restarted.snapshot().await.unwrap()).unwrap();
        assert_eq!(
            loaded["settings"]["reasoning_effort"],
            effort.unwrap_or("default")
        );
        assert_eq!(loaded["restart_required"], false);
    }
    let mut legacy: Value =
        serde_json::from_slice(&std::fs::read(settings_path(&config_path)).unwrap()).unwrap();
    if legacy.get("profiles").is_some() {
        legacy["profiles"][0]["config"]
            .as_object_mut()
            .unwrap()
            .remove("reasoning_effort");
    } else {
        legacy["config"]
            .as_object_mut()
            .unwrap()
            .remove("reasoning_effort");
    }
    std::fs::write(settings_path(&config_path), legacy.to_string()).unwrap();
    let loaded = serde_json::to_value(load_override(&config_path).unwrap().unwrap()).unwrap();
    assert_eq!(loaded["reasoning_effort"], "default");
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn connection_test_uses_the_same_clamped_native_effort_without_saving() {
    let seen = Arc::new(Mutex::new(None));
    let capture = seen.clone();
    let app = Router::new().route("/v1/chat/completions", post(move |Json(body): Json<Value>| {
        *capture.lock().unwrap() = Some(body);
        async { Json(json!({"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"{\"reply_to\":[],\"text\":\"你好\",\"topic\":null}"}}]})) }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let (status, result) = send(
        router(support::state()),
        "/api/llm/test",
        settings(&base, Some("ultra")),
    )
    .await;
    task.abort();
    assert_eq!(status, 200, "{result}");
    let body = seen.lock().unwrap().clone().unwrap();
    assert_eq!(body["reasoning_effort"], "high");
    assert_eq!(body["max_completion_tokens"], 4096);
    assert!(body.get("max_tokens").is_none());
}

#[test]
fn effort_and_cloud_budget_are_validated_without_weakening_local_limits() {
    for effort in [
        "default", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
    ] {
        let config: LlmConfig =
            serde_json::from_value(json!({"reasoning_effort":effort,"max_tokens":65536})).unwrap();
        assert!(config.validate().is_ok());
    }
    for config in [
        json!({"reasoning_effort":"invalid"}),
        json!({"max_tokens":65537}),
        json!({"mode":"local","base_url":"http://127.0.0.1:11434/v1","model":"local","api_key_env":"","max_retries":0,"max_tokens":1025}),
    ] {
        let config: LlmConfig = serde_json::from_value(config).unwrap();
        assert!(config.validate().is_err());
    }
}
