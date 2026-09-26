mod support;
use meowlive_server::{
    agent_settings::{AgentSettingsStore, settings_path},
    config::AppConfig,
    state::AppState,
    transport::http::router,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use support::{request, state_with_config};

struct Fixture {
    root: PathBuf,
    config: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("meowlive-agent-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let config = root.join("server.local.toml");
        std::fs::write(
            &config,
            "[agent]\npersona=\"TOML 人设\"\ncooldown_ms=30000\npending_capacity=17\nbatch_size=3\n",
        )
        .unwrap();
        Self { root, config }
    }
    fn state(&self) -> AppState {
        let mut state = state_with_config(AppConfig::load(&self.config).unwrap());
        state.agent_settings = Arc::new(AgentSettingsStore::new(settings_path(&self.config)));
        state
    }
    fn write_override(&self, text: &str) {
        let path = settings_path(&self.config);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn settings() -> Value {
    json!({
        "persona":"温柔猫咪\n简短回应",
        "system_prompt":"直接读出弹幕原文",
        "topic":"日常",
        "proactive_enabled":true,
        "cooldown_ms":45000,
        "interaction": {
            "chat_read_mode": "auto",
            "welcome_enabled": true,
            "busy_chat_count": 6,
            "busy_enter_count": 3,
            "busy_pending_count": 4,
            "welcome_cooldown_ms": 30000,
            "welcome_viewer_cooldown_ms": 600000
        }
    })
}

fn legacy_settings() -> Value {
    let mut value = settings();
    value["system_prompt"] = json!("");
    value
}

#[tokio::test]
async fn legacy_saved_settings_and_requests_default_optional_fields() {
    let fixture = Fixture::new();
    let legacy = json!({
        "persona": "温柔猫咪\n简短回应",
        "topic": "日常",
        "proactive_enabled": true,
        "cooldown_ms": 45000
    });
    let saved = json!({"schema":1,"settings":legacy}).to_string();
    fixture.write_override(&saved);
    assert_eq!(
        serde_json::to_value(fixture.state().agent_snapshot().await.settings).unwrap(),
        legacy_settings()
    );
    assert_eq!(
        std::fs::read_to_string(settings_path(&fixture.config)).unwrap(),
        saved
    );

    let (code, snapshot) = request(
        router(fixture.state()),
        "POST",
        "/api/agent/settings",
        legacy,
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(snapshot["settings"], legacy_settings());
    assert_eq!(
        serde_json::to_value(fixture.state().agent_snapshot().await.settings).unwrap(),
        legacy_settings()
    );
}

#[tokio::test]
async fn custom_interaction_preferences_survive_restart_without_resuming_agent() {
    let fixture = Fixture::new();
    let original = std::fs::read(&fixture.config).unwrap();
    for mode in ["all", "selective"] {
        let mut custom = settings();
        custom["interaction"] = json!({
            "chat_read_mode": mode,
            "welcome_enabled": false,
            "busy_chat_count": 12,
            "busy_enter_count": 5,
            "busy_pending_count": 8,
            "welcome_cooldown_ms": 45000,
            "welcome_viewer_cooldown_ms": 1200000
        });
        let (code, snapshot) = request(
            router(fixture.state()),
            "POST",
            "/api/agent/settings",
            custom.clone(),
        )
        .await;
        assert_eq!(code, 200);
        assert_eq!(snapshot["settings"], custom);
        let restarted = fixture.state().agent_snapshot().await;
        assert_eq!(serde_json::to_value(restarted.settings).unwrap(), custom);
        assert!(restarted.paused);
        let saved: Value =
            serde_json::from_slice(&std::fs::read(settings_path(&fixture.config)).unwrap())
                .unwrap();
        assert_eq!(saved["schema"], 2);
        assert_eq!(saved["settings"], custom);
        assert_eq!(saved["profiles"][0]["persona"], custom["persona"]);
        assert_eq!(saved["selected_profile_id"], saved["profiles"][0]["id"]);
        assert_eq!(std::fs::read(&fixture.config).unwrap(), original);
    }
}

#[tokio::test]
async fn invalid_interaction_limits_preserve_active_and_saved_settings() {
    let fixture = Fixture::new();
    let state = fixture.state();
    assert_eq!(
        request(
            router(state.clone()),
            "POST",
            "/api/agent/settings",
            settings()
        )
        .await
        .0,
        200
    );
    let previous = std::fs::read(settings_path(&fixture.config)).unwrap();
    for (field, value) in [
        ("busy_chat_count", 0),
        ("busy_chat_count", 1001),
        ("busy_enter_count", 0),
        ("busy_enter_count", 1001),
        ("busy_pending_count", 0),
        ("busy_pending_count", 513),
        ("welcome_cooldown_ms", 999),
        ("welcome_cooldown_ms", 3_600_001),
        ("welcome_viewer_cooldown_ms", 999),
        ("welcome_viewer_cooldown_ms", 86_400_001),
    ] {
        let mut invalid = settings();
        invalid["interaction"][field] = json!(value);
        let (code, _) = request(
            router(state.clone()),
            "POST",
            "/api/agent/settings",
            invalid,
        )
        .await;
        assert_eq!(code, 400, "invalid {field}={value} must be rejected");
        assert_eq!(
            std::fs::read(settings_path(&fixture.config)).unwrap(),
            previous
        );
        assert_eq!(
            serde_json::to_value(state.agent_snapshot().await.settings).unwrap(),
            settings()
        );
    }
}

#[tokio::test]
async fn saves_preferences_without_changing_toml_or_runtime_limits() {
    let fixture = Fixture::new();
    let original = std::fs::read(&fixture.config).unwrap();
    assert_eq!(
        fixture.state().agent_snapshot().await.settings.persona,
        "TOML 人设"
    );
    let (code, snapshot) = request(
        router(fixture.state()),
        "POST",
        "/api/agent/settings",
        settings(),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(snapshot["settings"], settings());
    assert_eq!(std::fs::read(&fixture.config).unwrap(), original);
    let config = AppConfig::load(&fixture.config).unwrap();
    assert_eq!(config.agent.pending_capacity, 17);
    assert_eq!(config.agent.batch_size, 3);
    let restored = fixture.state().agent_snapshot().await;
    assert_eq!(serde_json::to_value(restored.settings).unwrap(), settings());
    assert!(restored.paused);
    let path = settings_path(&fixture.config);
    let saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["schema"], 2);
    assert_eq!(saved["settings"], settings());
    assert_eq!(saved["profiles"][0]["persona"], settings()["persona"]);
    assert_eq!(saved["selected_profile_id"], saved["profiles"][0]["id"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    // Each TOML configuration has its own local preferences.
    let other = fixture.root.join("offline.local.toml");
    std::fs::write(&other, &original).unwrap();
    assert_eq!(AppConfig::load(&other).unwrap().agent.persona, "TOML 人设");
}

#[tokio::test]
async fn invalid_request_preserves_saved_and_active_settings() {
    let fixture = Fixture::new();
    let state = fixture.state();
    assert_eq!(
        request(
            router(state.clone()),
            "POST",
            "/api/agent/settings",
            settings()
        )
        .await
        .0,
        200
    );
    let previous = std::fs::read(settings_path(&fixture.config)).unwrap();
    for invalid in [
        json!({"persona":"","topic":"","proactive_enabled":true,"cooldown_ms":45000}),
        json!({"persona":"新的人设","topic":"","proactive_enabled":true,"cooldown_ms":0}),
    ] {
        assert_eq!(
            request(
                router(state.clone()),
                "POST",
                "/api/agent/settings",
                invalid
            )
            .await
            .0,
            400
        );
        assert_eq!(
            std::fs::read(settings_path(&fixture.config)).unwrap(),
            previous
        );
        assert_eq!(
            serde_json::to_value(state.agent_snapshot().await.settings).unwrap(),
            settings()
        );
    }
}

#[tokio::test]
async fn write_failure_is_visible_and_preserves_active_settings() {
    let fixture = Fixture::new();
    let state = fixture.state();
    let before = state.agent_snapshot().await;
    // Deterministic even when tests run as root: the intended directory is a file.
    std::fs::write(fixture.root.join("local"), "occupied").unwrap();
    let (code, error) = request(
        router(state.clone()),
        "POST",
        "/api/agent/settings",
        settings(),
    )
    .await;
    assert_eq!(code, 500);
    assert_eq!(error["code"], "agent_settings_save_failed");
    assert_eq!(state.agent_snapshot().await, before);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("local")).unwrap(),
        "occupied"
    );
}

#[tokio::test]
async fn failed_replacement_removes_temporary_file_and_preserves_active_settings() {
    let fixture = Fixture::new();
    let state = fixture.state();
    let before = state.agent_snapshot().await;
    let path = settings_path(&fixture.config);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("existing"), "unchanged").unwrap();
    let (code, _) = request(
        router(state.clone()),
        "POST",
        "/api/agent/settings",
        settings(),
    )
    .await;
    assert_eq!(code, 500);
    assert_eq!(state.agent_snapshot().await, before);
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        1
    );
    assert_eq!(
        std::fs::read_to_string(path.join("existing")).unwrap(),
        "unchanged"
    );
}

#[test]
fn invalid_local_files_are_reported_without_silently_resetting_or_overwriting_them() {
    let fixture = Fixture::new();
    for text in [
        "{".to_string(),
        json!({"schema":2,"settings":settings()}).to_string(),
        json!({"schema":1,"settings":{"persona":"","topic":"","proactive_enabled":false,"cooldown_ms":30000}}).to_string(),
        " ".repeat(16385),
    ] {
        fixture.write_override(&text);
        assert!(AppConfig::load(&fixture.config).unwrap_err().contains("Agent 配置"));
        assert_eq!(std::fs::read_to_string(settings_path(&fixture.config)).unwrap(), text);
    }
}

#[tokio::test]
async fn concurrent_saves_keep_last_persisted_and_active_settings_consistent() {
    let fixture = Fixture::new();
    let state = fixture.state();
    let mut requests = tokio::task::JoinSet::new();
    for index in 0..8 {
        let state = state.clone();
        requests.spawn(async move {
            let mut value = settings();
            value["persona"] = json!(format!("人设 {index}"));
            request(router(state), "POST", "/api/agent/settings", value)
                .await
                .0
        });
    }
    while let Some(result) = requests.join_next().await {
        assert_eq!(result.unwrap(), 200);
    }
    assert_eq!(
        fixture.state().agent_snapshot().await.settings,
        state.agent_snapshot().await.settings
    );
}
