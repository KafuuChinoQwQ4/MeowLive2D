mod support;

use axum::{
    Json, Router,
    body::Body,
    http::{HeaderMap, Request},
    routing::{get, post},
};
use meowlive_protocol::llm::{LlmSettings, LlmSettingsRequest};
use meowlive_server::{
    config::LlmConfig,
    llm_settings::{LlmSettingsStore, settings_path},
    transport::http::router,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("meow-model-list-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        settings_path(&self.0.join("server.toml"))
    }
    fn state(&self) -> meowlive_server::state::AppState {
        let mut state = support::state();
        state.llm_settings = Arc::new(LlmSettingsStore::new(
            LlmConfig::default(),
            Some(self.file()),
        ));
        state
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn upstream(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, task)
}

fn draft(base: &str, key: Option<&str>) -> Value {
    json!({"provider":"custom","api_format":"openai_chat","base_url":base,"mode":"cloud","api_key":key,"clear_api_key":false})
}

fn settings(base: &str, key: Option<&str>) -> LlmSettingsRequest {
    LlmSettingsRequest {
        settings: LlmSettings {
            provider: "custom".into(),
            api_format: "openai_chat".into(),
            base_url: base.into(),
            model: "picked-model".into(),
            mode: "cloud".into(),
            timeout_seconds: 10,
            max_tokens: 256,
            json_mode: true,
            reasoning_effort: "default".into(),
        },
        api_key: key.map(str::to_owned),
        clear_api_key: false,
    }
}

#[tokio::test]
async fn fetches_without_a_model_or_saving_and_tests_the_selected_model() {
    let (base, task) = upstream(Router::new()
        .route("/v1/models", get(|headers: HeaderMap| async move {
            assert_eq!(headers["authorization"], "Bearer draft-catalog-key");
            Json(json!({"data":[{"id":"picked-model"},{"id":"another-model"}]}))
        }))
        .route("/v1/chat/completions", post(|headers: HeaderMap, Json(body): Json<Value>| async move {
            assert_eq!(headers["authorization"], "Bearer draft-catalog-key");
            assert_eq!(body["model"], "picked-model");
            Json(json!({"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"{\"reply_to\":[],\"text\":\"你好\",\"topic\":null}"}}]}))
        }))).await;
    let fixture = Fixture::new();
    let state = fixture.state();
    let (status, result) = request(
        router(state.clone()),
        "POST",
        "/api/llm/models",
        draft(&base, Some("draft-catalog-key")),
    )
    .await;
    assert_eq!(status, 200, "{result}");
    assert_eq!(result["base_url"], format!("{base}/v1"));
    assert_eq!(
        result["models"],
        json!([{"id":"another-model","name":"another-model"},{"id":"picked-model","name":"picked-model"}])
    );
    assert!(!result.to_string().contains("draft-catalog-key"));
    assert!(!fixture.file().exists());
    let selected = settings(
        result["base_url"].as_str().unwrap(),
        Some("draft-catalog-key"),
    );
    let (status, result) = request(
        router(state),
        "POST",
        "/api/llm/test",
        serde_json::to_value(selected).unwrap(),
    )
    .await;
    assert_eq!(status, 200, "{result}");
    assert!(!fixture.file().exists());
    task.abort();
}

#[tokio::test]
async fn saved_key_follows_same_api_base_but_not_another_origin_or_prefix() {
    let (base, task) = upstream(
        Router::new()
            .route(
                "/v1/models",
                get(|headers: HeaderMap| async move {
                    assert_eq!(headers["authorization"], "Bearer saved-catalog-key");
                    Json(json!({"data":[{"id":"picked-model"}]}))
                }),
            )
            .route(
                "/other/models",
                get(|headers: HeaderMap| async move {
                    assert!(!headers.contains_key("authorization"));
                    Json(json!({"data":[]}))
                }),
            ),
    )
    .await;
    let (other, other_task) = upstream(Router::new().route(
        "/v1/models",
        get(|headers: HeaderMap| async move {
            assert!(!headers.contains_key("authorization"));
            Json(json!({"data":[]}))
        }),
    ))
    .await;
    let fixture = Fixture::new();
    let state = fixture.state();
    state
        .llm_settings
        .save(settings(
            &format!("{base}/v1/chat/completions"),
            Some("saved-catalog-key"),
        ))
        .await
        .unwrap();
    let before = std::fs::read(fixture.file()).unwrap();
    for url in [format!("{base}/v1"), format!("{base}/other"), other] {
        let (status, value) = request(
            router(state.clone()),
            "POST",
            "/api/llm/models",
            draft(&url, None),
        )
        .await;
        assert_eq!(status, 200, "{value}");
        assert!(!value.to_string().contains("saved-catalog-key"));
        assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    }
    let candidate = state
        .llm_settings
        .candidate(settings(&format!("{base}/v1"), None))
        .await
        .unwrap();
    assert_eq!(candidate.api_key.as_deref(), Some("saved-catalog-key"));
    task.abort();
    other_task.abort();
}

#[tokio::test]
async fn discovery_rejects_invalid_local_targets_and_foreign_origins() {
    let fixture = Fixture::new();
    let state = fixture.state();
    for body in [
        json!({"provider":"custom","api_format":"unknown","base_url":"https://example.test/v1","mode":"cloud","api_key":null,"clear_api_key":false}),
        draft("https://key@example.test/v1", None),
        draft("https://example.test/v1?key=private", None),
        draft("https://example.test/v1#fragment", None),
        {
            let mut v = draft("https://example.test/v1", None);
            v["mode"] = json!("local");
            v
        },
        {
            let mut v = draft("http://127.0.0.1:1/v1", Some("private"));
            v["mode"] = json!("local");
            v
        },
        {
            let mut v = draft("http://127.0.0.1:1/v1", Some("private"));
            v["clear_api_key"] = json!(true);
            v
        },
    ] {
        let (status, _) = request(router(state.clone()), "POST", "/api/llm/models", body).await;
        assert_eq!(status, 400);
    }
    let response = router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/llm/models")
                .header("origin", "https://foreign.example")
                .header("content-type", "application/json")
                .body(Body::from(
                    draft("https://example.test/v1", None).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert!(!fixture.file().exists());
}

#[tokio::test]
async fn model_discovery_and_connection_testing_share_a_bounded_request_slot() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (base, task) = upstream(Router::new().route(
        "/v1/models",
        get({
            let entered = entered.clone();
            let release = release.clone();
            move || {
                let entered = entered.clone();
                let release = release.clone();
                async move {
                    entered.notify_one();
                    release.notified().await;
                    Json(json!({"data":[{"id":"picked-model"}]}))
                }
            }
        }),
    ))
    .await;
    let fixture = Fixture::new();
    let state = fixture.state();
    let first = tokio::spawn(request(
        router(state.clone()),
        "POST",
        "/api/llm/models",
        draft(&base, None),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    for (path, body) in [
        ("/api/llm/models", draft(&base, None)),
        (
            "/api/llm/test",
            serde_json::to_value(settings(&base, None)).unwrap(),
        ),
    ] {
        let (status, _) = request(router(state.clone()), "POST", path, body).await;
        assert_eq!(status, 409);
    }
    release.notify_one();
    assert_eq!(first.await.unwrap().0, 200);
    assert!(!fixture.file().exists());
    task.abort();
}

#[tokio::test]
async fn discovery_reports_provider_failure_without_body_or_secret_and_preserves_config() {
    let (base, task) = upstream(Router::new().route(
        "/v1/models",
        get(|| async {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                "private-upstream-detail private-discovery-key",
            )
        }),
    ))
    .await;
    let fixture = Fixture::new();
    let state = fixture.state();
    state
        .llm_settings
        .save(settings(
            &format!("{base}/v1"),
            Some("private-discovery-key"),
        ))
        .await
        .unwrap();
    let before = std::fs::read(fixture.file()).unwrap();
    let (status, result) = request(
        router(state),
        "POST",
        "/api/llm/models",
        draft(&format!("{base}/v1"), None),
    )
    .await;
    assert_eq!(status, 502);
    assert_eq!(result["code"], "llm_models_failed");
    assert!(!result.to_string().contains("private-"));
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    task.abort();
}

#[tokio::test]
async fn unversioned_saved_connection_keeps_key_through_discovery_test_and_save() {
    let (base, task) = upstream(Router::new()
        .route("/models", get(|headers: HeaderMap| async move {
            assert_eq!(headers["authorization"], "Bearer unversioned-key");
            Json(json!({"data":[{"id":"picked-model"}]}))
        }))
        .route("/v1/models", get(|headers: HeaderMap| async move {
            assert!(!headers.contains_key("authorization"), "saved root key must not be probed at a new prefix");
            axum::http::StatusCode::NOT_FOUND
        }))
        .route("/chat/completions", post(|headers: HeaderMap| async move {
            assert_eq!(headers["authorization"],"Bearer unversioned-key");
            Json(json!({"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"{\"reply_to\":[],\"text\":\"你好\",\"topic\":null}"}}]}))
        }))).await;
    let fixture = Fixture::new();
    let state = fixture.state();
    state
        .llm_settings
        .save(settings(
            &format!("{base}/chat/completions"),
            Some("unversioned-key"),
        ))
        .await
        .unwrap();
    for url in [format!("{base}/chat/completions"), base.clone()] {
        let (status, result) = request(
            router(state.clone()),
            "POST",
            "/api/llm/models",
            draft(&url, None),
        )
        .await;
        assert_eq!(status, 200, "{result}");
        assert_eq!(result["base_url"], base);
        let selection =
            serde_json::to_value(settings(result["base_url"].as_str().unwrap(), None)).unwrap();
        let (status, result) = request(
            router(state.clone()),
            "POST",
            "/api/llm/test",
            selection.clone(),
        )
        .await;
        assert_eq!(status, 200, "{result}");
        let (status, result) = request(
            router(state.clone()),
            "POST",
            "/api/llm/settings",
            selection,
        )
        .await;
        assert_eq!(status, 200, "{result}");
        assert_eq!(result["key_configured"], true);
    }
    task.abort();
}

#[tokio::test]
async fn changing_versioned_connection_to_root_never_sends_retained_key_on_fallback() {
    let (base, task) = upstream(
        Router::new()
            .route(
                "/v1/models",
                get(|headers: HeaderMap| async move {
                    assert!(!headers.contains_key("authorization"));
                    axum::http::StatusCode::NOT_FOUND
                }),
            )
            .route(
                "/models",
                get(|headers: HeaderMap| async move {
                    assert!(!headers.contains_key("authorization"));
                    Json(json!({"data":[]}))
                }),
            ),
    )
    .await;
    let fixture = Fixture::new();
    let state = fixture.state();
    state
        .llm_settings
        .save(settings(&format!("{base}/v1"), Some("version-scoped-key")))
        .await
        .unwrap();
    let (status, result) =
        request(router(state), "POST", "/api/llm/models", draft(&base, None)).await;
    assert_eq!(status, 200, "{result}");
    task.abort();
}

async fn request(app: Router, method: &str, path: &str, body: Value) -> (u16, Value) {
    use http_body_util::BodyExt;
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
