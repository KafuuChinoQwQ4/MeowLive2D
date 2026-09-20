use meowlive_server::config::{AppConfig, LlmConfig};

#[test]
fn llm_formats_are_explicit_and_unknown_formats_are_rejected() {
    for format in [
        "openai_chat",
        "openai_responses",
        "anthropic_messages",
        "gemini_generate_content",
    ] {
        let parsed = AppConfig::parse(&format!("[llm]\napi_format = \"{format}\"\n"));
        assert!(parsed.is_ok(), "{format}: {parsed:?}");
    }
    assert!(AppConfig::parse("[llm]\napi_format = \"unknown\"\n").is_err());
    assert_eq!(LlmConfig::default().api_format, "openai_chat");
}

mod support;
use axum::{body::Body, http::Request};
use meowlive_server::transport::http::router;
use tower::ServiceExt;

#[tokio::test]
async fn configuration_page_can_read_llm_settings_without_exposing_keys() {
    let response = router(support::state())
        .oneshot(
            Request::builder()
                .uri("/api/llm/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}

use meowlive_protocol::llm::{LlmSettings, LlmSettingsRequest};
use meowlive_server::llm_settings::{LlmSettingsStore, settings_path};
use std::sync::Arc;

fn request(key: Option<&str>) -> LlmSettingsRequest {
    LlmSettingsRequest {
        settings: LlmSettings {
            provider: "custom".into(),
            api_format: "openai_chat".into(),
            base_url: "https://llm.example.test/v1".into(),
            model: "test-model".into(),
            mode: "cloud".into(),
            timeout_seconds: 10,
            max_tokens: 256,
            json_mode: true,
        },
        api_key: key.map(str::to_owned),
        clear_api_key: false,
    }
}
struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("meow-llm-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn config_path(&self) -> std::path::PathBuf {
        self.0.join("server.local.toml")
    }
    fn store(&self) -> LlmSettingsStore {
        LlmSettingsStore::new(
            LlmConfig::default(),
            Some(settings_path(&self.config_path())),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn saves_private_key_and_loads_it_after_restart_without_returning_it() {
    let fixture = Fixture::new();
    std::fs::write(fixture.config_path(), "").unwrap();
    let store = fixture.store();
    let snapshot = store.save(request(Some("private-test-key"))).await.unwrap();
    assert!(snapshot.restart_required && snapshot.key_configured);
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("private-test-key")
    );
    let loaded = AppConfig::load(&fixture.config_path()).unwrap();
    assert_eq!(loaded.llm.api_key.as_deref(), Some("private-test-key"));
    assert_eq!(loaded.llm.model, "test-model");
    assert!(!format!("{loaded:?}").contains("private-test-key"));
    let restarted = LlmSettingsStore::new(loaded.llm, Some(settings_path(&fixture.config_path())));
    assert!(!restarted.snapshot().await.unwrap().restart_required);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(settings_path(&fixture.config_path()))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[tokio::test]
async fn retained_key_is_bound_to_destination_and_can_be_cleared() {
    let fixture = Fixture::new();
    let store = fixture.store();
    store.save(request(Some("retained-key"))).await.unwrap();
    let mut same = request(None);
    same.settings.model = "another-model".into();
    assert!(store.save(same).await.unwrap().key_configured);
    let mut changed = request(None);
    changed.settings.base_url = "https://another.example.test/v1".into();
    assert!(!store.save(changed).await.unwrap().key_configured);
    store.save(request(Some("replacement-key"))).await.unwrap();
    let mut clear = request(None);
    clear.clear_api_key = true;
    assert!(!store.save(clear).await.unwrap().key_configured);
}

#[tokio::test]
async fn rejected_settings_never_replace_last_good_configuration() {
    let fixture = Fixture::new();
    let store = fixture.store();
    store.save(request(Some("valid-key"))).await.unwrap();
    let before = std::fs::read(settings_path(&fixture.config_path())).unwrap();
    for url in [
        "https://host.test/v1?key=secret",
        "https://secret@host.test/v1",
        "file:///tmp/test",
    ] {
        let mut invalid = request(None);
        invalid.settings.base_url = url.into();
        assert!(store.save(invalid).await.is_err());
        assert_eq!(
            std::fs::read(settings_path(&fixture.config_path())).unwrap(),
            before
        );
    }
}

#[tokio::test]
async fn http_save_returns_metadata_and_foreign_origin_cannot_change_it() {
    let fixture = Fixture::new();
    let mut state = support::state();
    state.llm_settings = Arc::new(fixture.store());
    let body = serde_json::to_value(request(Some("http-test-key"))).unwrap();
    let (status, value) = support::request(
        router(state.clone()),
        "POST",
        "/api/llm/settings",
        body.clone(),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(value["key_configured"], true);
    assert!(!value.to_string().contains("http-test-key"));
    let response = router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/llm/settings")
                .header("origin", "https://foreign.example")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn connection_test_uses_draft_without_saving_it_or_echoing_provider_text() {
    use axum::{Json, Router, routing::post};
    use serde_json::{Value, json};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let upstream = tokio::spawn(async move {
        axum::serve(listener, Router::new().route("/v1/chat/completions", post(|Json(body): Json<Value>| async move {
            assert_eq!(body["model"], "draft-model");
            Json(json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"reply_to\":[],\"text\":\"private-provider-output\",\"topic\":null}"}}]}))
        }))).await.unwrap();
    });
    let fixture = Fixture::new();
    let mut state = support::state();
    state.llm_settings = Arc::new(fixture.store());
    let mut draft = request(Some("draft-key"));
    draft.settings.base_url = format!("http://{address}/v1");
    draft.settings.model = "draft-model".into();
    let (status, value) = support::request(
        router(state),
        "POST",
        "/api/llm/test",
        serde_json::to_value(draft).unwrap(),
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert!(!value.to_string().contains("private-provider-output"));
    assert!(!settings_path(&fixture.config_path()).exists());
    upstream.abort();
}

#[allow(dead_code)]
mod process_support;
#[tokio::test]
async fn missing_environment_key_does_not_block_opening_configuration_page() {
    let process = process_support::ServerProcess::start(&format!("[server]\nlisten_address='127.0.0.1:0'\n[llm]\nbase_url='https://example.test/v1'\nmodel='test-model'\napi_key_env='MISSING_{}'\n", uuid::Uuid::new_v4().simple())).await;
    let client = reqwest::Client::new();
    let settings: serde_json::Value = client
        .get(format!("{}/api/llm/settings", process.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(settings["key_configured"], false);
    assert_eq!(settings["storage_available"], true);
    let agent: serde_json::Value = client
        .get(format!("{}/api/agent", process.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(agent["llm_configured"], false);
}

#[tokio::test]
async fn bootstrap_selects_the_configured_protocol_for_agent_decisions() {
    use axum::{Json, Router, routing::post};
    use meowlive_application::ports::llm::DecisionRequest;
    use serde_json::json;
    for (format, path, response) in [
        (
            "anthropic_messages",
            "/v1/messages",
            json!({"type":"message","role":"assistant","stop_reason":"end_turn","content":[{"type":"text","text":"{\"reply_to\":[],\"text\":\"你好\",\"topic\":null}"}]}),
        ),
        (
            "gemini_generate_content",
            "/v1beta/models/test-model:generateContent",
            json!({"candidates":[{"finishReason":"STOP","content":{"role":"model","parts":[{"text":"{\"reply_to\":[],\"text\":\"你好\",\"topic\":null}"}]}}]}),
        ),
        (
            "openai_responses",
            "/v1/responses",
            json!({"status":"completed","output":[{"type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"{\"reply_to\":[],\"text\":\"你好\",\"topic\":null}"}]}]}),
        ),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(path, post(move || async move { Json(response) })),
            )
            .await
            .unwrap();
        });
        let model = meowlive_server::bootstrap::build_model(&LlmConfig {
            api_format: format.into(),
            base_url: format!(
                "http://{address}{}",
                if format == "gemini_generate_content" {
                    "/v1beta"
                } else {
                    "/v1"
                }
            ),
            model: "test-model".into(),
            api_key_env: String::new(),
            api_key: Some("test-key".into()),
            ..LlmConfig::default()
        })
        .unwrap()
        .unwrap();
        let reply = model
            .decide(DecisionRequest {
                memory_context: vec![],
                persona: "测试主播".into(),
                topic: String::new(),
                history: vec![],
                events: vec![],
            })
            .await
            .unwrap();
        assert_eq!(reply.text.as_deref(), Some("你好"), "{format}");
        upstream.abort();
    }
}

#[tokio::test]
async fn saving_local_llm_rejects_remote_tts_before_breaking_next_startup() {
    let fixture = Fixture::new();
    let mut state = support::state();
    state.llm_settings = Arc::new(fixture.store());
    Arc::make_mut(&mut state.config).speech.base_url = "http://192.0.2.1:9880".into();
    let mut local = request(None);
    local.settings.mode = "local".into();
    local.settings.base_url = "http://127.0.0.1:8080/v1".into();
    let (status, _) = support::request(
        router(state),
        "POST",
        "/api/llm/settings",
        serde_json::to_value(local).unwrap(),
    )
    .await;
    assert_eq!(status, 400);
    assert!(!settings_path(&fixture.config_path()).exists());
}
