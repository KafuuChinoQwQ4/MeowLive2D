mod support;
use axum::{Router, body::Body, http::Request};
use http_body_util::BodyExt;
use meowlive_server::{
    agent_settings::{AgentSettingsStore, settings_path},
    config::AppConfig,
    state::AppState,
    transport::http::router,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use support::state_with_config;
use tower::ServiceExt;

async fn request(app: Router, method: &str, path: &str, body: Value) -> (u16, Value) {
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
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

struct Fixture {
    root: PathBuf,
    config: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("meowlive-personas-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let config = root.join("server.toml");
        std::fs::write(&config, "[agent]\npersona=\"原始人设\"\n").unwrap();
        Self { root, config }
    }

    fn state(&self) -> AppState {
        let mut state = state_with_config(AppConfig::load(&self.config).unwrap());
        state.agent_settings = Arc::new(AgentSettingsStore::new(settings_path(&self.config)));
        state
    }

    fn saved(&self) -> Value {
        serde_json::from_slice(&std::fs::read(settings_path(&self.config)).unwrap()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn creates_and_switches_personas_without_losing_interaction_settings() {
    let fixture = Fixture::new();
    let state = fixture.state();
    let (code, initial) = request(
        router(state.clone()),
        "GET",
        "/api/agent/personas",
        json!({}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(initial["profiles"], json!([]));
    assert_eq!(initial["selected_profile_id"], Value::Null);
    assert_eq!(initial["storage_available"], true);

    let mut settings = serde_json::to_value(state.agent_snapshot().await.settings).unwrap();
    settings["topic"] = json!("夜间电台");
    settings["cooldown_ms"] = json!(45000);
    assert_eq!(
        request(
            router(state.clone()),
            "POST",
            "/api/agent/settings",
            settings,
        )
        .await
        .0,
        200
    );
    let first = request(
        router(state.clone()),
        "GET",
        "/api/agent/personas",
        json!({}),
    )
    .await
    .1;
    let first_id = first["selected_profile_id"].as_str().unwrap().to_owned();
    assert_eq!(first["profiles"][0]["persona"], "原始人设");

    let (code, second) = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas",
        json!({"name":"魔女", "persona":"魔女主播"}),
    )
    .await;
    assert_eq!(code, 200);
    let second_id = second["selected_profile_id"].as_str().unwrap().to_owned();
    assert_ne!(first_id, second_id);
    assert_eq!(second["profiles"].as_array().unwrap().len(), 2);
    let current = state.agent_snapshot().await;
    assert_eq!(current.settings.persona, "魔女主播");
    assert_eq!(current.settings.topic, "夜间电台");
    assert_eq!(current.settings.cooldown_ms, 45000);
    assert!(current.paused);

    let (code, selected) = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas/select",
        json!({"id":first_id}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(selected["selected_profile_id"], first_id);
    assert_eq!(state.agent_snapshot().await.settings.persona, "原始人设");
    assert_eq!(
        fixture.state().agent_snapshot().await.settings.persona,
        "原始人设"
    );
    assert_eq!(fixture.saved()["schema"], 2);
}

#[tokio::test]
async fn selected_persona_follows_settings_save_and_profile_rename_delete() {
    let fixture = Fixture::new();
    let state = fixture.state();
    let first = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas",
        json!({"name":"角色 A", "persona":"设定 A"}),
    )
    .await
    .1;
    let first_id = first["selected_profile_id"].as_str().unwrap().to_owned();
    let second = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas",
        json!({"name":"角色 B", "persona":"设定 B"}),
    )
    .await
    .1;
    let second_id = second["selected_profile_id"].as_str().unwrap().to_owned();

    let mut settings = serde_json::to_value(state.agent_snapshot().await.settings).unwrap();
    settings["persona"] = json!("设定 B 更新");
    assert_eq!(
        request(
            router(state.clone()),
            "POST",
            "/api/agent/settings",
            settings
        )
        .await
        .0,
        200
    );
    let updated = request(
        router(state.clone()),
        "GET",
        "/api/agent/personas",
        json!({}),
    )
    .await
    .1;
    assert_eq!(updated["profiles"][1]["persona"], "设定 B 更新");
    assert_eq!(updated["profiles"][0]["persona"], "设定 A");

    let (code, renamed) = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas/rename",
        json!({"id":second_id, "name":"新名字"}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(renamed["profiles"][1]["name"], "新名字");
    let (code, deleted) = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas/delete",
        json!({"id":second_id}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(deleted["selected_profile_id"], first_id);
    assert_eq!(state.agent_snapshot().await.settings.persona, "设定 A");
    assert_eq!(
        fixture.state().agent_snapshot().await.settings.persona,
        "设定 A"
    );

    let (code, empty) = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas/delete",
        json!({"id":first_id}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(empty["profiles"], json!([]));
    assert_eq!(empty["selected_profile_id"], Value::Null);
    assert_eq!(state.agent_snapshot().await.settings.persona, "设定 A");
}

#[tokio::test]
async fn legacy_single_persona_is_migrated_on_write_without_losing_settings() {
    let fixture = Fixture::new();
    let path = settings_path(&fixture.config);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut settings =
        serde_json::to_value(fixture.state().agent_snapshot().await.settings).unwrap();
    settings["persona"] = json!("旧人设");
    settings["topic"] = json!("原有话题");
    let legacy = json!({"schema":1,"settings":settings}).to_string();
    std::fs::write(&path, &legacy).unwrap();
    let state = fixture.state();
    let (code, before) = request(
        router(state.clone()),
        "GET",
        "/api/agent/personas",
        json!({}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(before["profiles"][0]["persona"], "旧人设");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), legacy);
    let id = before["selected_profile_id"].as_str().unwrap();
    let (code, after) = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas/rename",
        json!({"id":id,"name":"保留角色"}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(after["profiles"][0]["name"], "保留角色");
    assert_eq!(fixture.saved()["schema"], 2);
    assert_eq!(fixture.saved()["settings"]["topic"], "原有话题");
    assert_eq!(
        fixture.state().agent_snapshot().await.settings.persona,
        "旧人设"
    );
}

#[tokio::test]
async fn invalid_profile_change_does_not_change_active_or_saved_persona() {
    let fixture = Fixture::new();
    let state = fixture.state();
    let created = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas",
        json!({"name":"角色", "persona":"有效设定"}),
    )
    .await
    .1;
    let id = created["selected_profile_id"].as_str().unwrap().to_owned();
    let before = std::fs::read(settings_path(&fixture.config)).unwrap();
    for (path, body) in [
        (
            "/api/agent/personas",
            json!({"name":"坏角色", "persona":""}),
        ),
        ("/api/agent/personas/select", json!({"id":"missing"})),
        ("/api/agent/personas/rename", json!({"id":id, "name":""})),
    ] {
        assert_eq!(
            request(router(state.clone()), "POST", path, body).await.0,
            400
        );
        assert_eq!(
            std::fs::read(settings_path(&fixture.config)).unwrap(),
            before
        );
        assert_eq!(state.agent_snapshot().await.settings.persona, "有效设定");
    }
}

#[tokio::test]
async fn stale_editor_cannot_overwrite_a_newly_selected_persona() {
    let fixture = Fixture::new();
    let state = fixture.state();
    let first = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas",
        json!({"name":"角色 A", "persona":"设定 A"}),
    )
    .await
    .1;
    let first_id = first["selected_profile_id"].as_str().unwrap().to_owned();
    let second = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas",
        json!({"name":"角色 B", "persona":"设定 B"}),
    )
    .await
    .1;
    let second_id = second["selected_profile_id"].as_str().unwrap().to_owned();
    let before = std::fs::read(settings_path(&fixture.config)).unwrap();

    let (code, _) = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas/update",
        json!({"id":first_id, "name":"旧页面改名", "persona":"错误覆盖"}),
    )
    .await;
    assert_eq!(code, 409);
    assert_eq!(
        std::fs::read(settings_path(&fixture.config)).unwrap(),
        before
    );
    assert_eq!(state.agent_snapshot().await.settings.persona, "设定 B");

    let (code, updated) = request(
        router(state.clone()),
        "POST",
        "/api/agent/personas/update",
        json!({"id":second_id, "name":"角色 B 更新", "persona":"设定 B 更新"}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(updated["profiles"][1]["name"], "角色 B 更新");
    assert_eq!(updated["profiles"][1]["persona"], "设定 B 更新");
    assert_eq!(
        fixture.state().agent_snapshot().await.settings.persona,
        "设定 B 更新"
    );
}
