mod support;

use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use meowlive_server::transport::http::router;
use meowlive_server::{
    config::AppConfig,
    live_settings::{LiveSettingsStore, settings_path},
    state::AppState,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("meow-live-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        let fixture = Self(path);
        std::fs::write(fixture.config_path(), "").unwrap();
        fixture
    }
    fn config_path(&self) -> PathBuf {
        self.0.join("server.local.toml")
    }
    fn state(&self) -> AppState {
        let mut state = support::state();
        state.live_settings = Arc::new(LiveSettingsStore::new(settings_path(&self.config_path())));
        state
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn draft() -> Value {
    json!({"enabled": true, "app_id": "9007199254740993", "access_key_id": "private-key-id", "access_key_secret": "private-key-secret", "identity_code": "private-anchor-code", "clear_credentials": false})
}

async fn save(state: &AppState, draft: Value) -> (u16, Value) {
    support::request(router(state.clone()), "POST", "/api/live/settings", draft).await
}

#[tokio::test]
async fn unconfigured_live_connection_can_open_the_setup_form() {
    let response = router(support::state())
        .oneshot(
            Request::builder()
                .uri("/api/live/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["enabled"], false);
    assert_eq!(value["app_id"], "");
    assert_eq!(value["access_key_id_configured"], false);
    assert_eq!(value["access_key_secret_configured"], false);
    assert_eq!(value["identity_code_configured"], false);
    assert_eq!(value["storage_available"], false);
}

#[tokio::test]
async fn saving_credentials_makes_connection_available_and_survives_reload_without_echoing_secrets()
{
    let fixture = Fixture::new();
    let state = fixture.state();
    let (status, value) = save(&state, draft()).await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["app_id"], "9007199254740993");
    assert_eq!(value["access_key_secret_configured"], true);
    assert_eq!(value["storage_available"], true);
    assert!(!value.to_string().contains("private-"));
    let snapshot = state.live_snapshot().await;
    assert!(snapshot.configured);
    assert_eq!(
        snapshot.phase,
        meowlive_protocol::live::LiveConnectionPhase::Disconnected
    );
    let loaded = AppConfig::load(&fixture.config_path()).unwrap();
    assert_eq!(loaded.live.app_id, 9007199254740993);
    assert_eq!(
        loaded
            .live
            .resolved_credentials()
            .access_key_secret
            .as_deref(),
        Some("private-key-secret")
    );
    assert!(
        meowlive_server::bootstrap::build_live_source(&loaded.live)
            .unwrap()
            .is_some()
    );
    assert!(!format!("{loaded:?}").contains("private-"));
    let (_, readback) =
        support::request(router(state), "GET", "/api/live/settings", json!(null)).await;
    assert_eq!(readback, value);
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
async fn blank_fields_preserve_saved_credentials_and_explicit_clear_disables_without_env_fallback()
{
    let fixture = Fixture::new();
    let state = fixture.state();
    assert_eq!(save(&state, draft()).await.0, 200);
    let mut retain = json!({"enabled": true, "app_id": "9007199254740993", "access_key_id": null, "access_key_secret": "", "identity_code": null});
    assert_eq!(save(&state, retain.clone()).await.0, 200);
    assert_eq!(
        AppConfig::load(&fixture.config_path())
            .unwrap()
            .live
            .resolved_credentials()
            .identity_code
            .as_deref(),
        Some("private-anchor-code")
    );
    retain["enabled"] = json!(false);
    retain["clear_credentials"] = json!(true);
    let (status, snapshot) = save(&state, retain).await;
    assert_eq!(status, 200, "{snapshot}");
    assert_eq!(snapshot["identity_code_configured"], false);
    let mut config = AppConfig::load(&fixture.config_path()).unwrap().live;
    config.identity_code_env = "PATH".into();
    assert!(config.resolved_credentials().identity_code.is_none());
    assert!(!state.live_snapshot().await.configured);
    assert_eq!(
        state.live_snapshot().await.phase,
        meowlive_protocol::live::LiveConnectionPhase::Disabled
    );
    assert!(
        !std::fs::read_to_string(settings_path(&fixture.config_path()))
            .unwrap()
            .contains("private-")
    );
}

#[tokio::test]
async fn invalid_settings_never_overwrite_saved_credentials_or_change_the_active_source() {
    let fixture = Fixture::new();
    let state = fixture.state();
    assert_eq!(save(&state, draft()).await.0, 200);
    let path = settings_path(&fixture.config_path());
    let before = std::fs::read(&path).unwrap();
    let mut invalids = vec![];
    for id in [
        "0",
        "-1",
        "1e3",
        "9223372036854775808",
        "18446744073709551616",
    ] {
        let mut invalid = draft();
        invalid["app_id"] = json!(id);
        invalids.push(invalid);
    }
    let mut invalid = draft();
    invalid["access_key_secret"] = json!("x".repeat(513));
    invalids.push(invalid);
    let mut invalid = draft();
    invalid["access_key_id"] = json!("invalid\nheader");
    invalids.push(invalid);
    let mut invalid = draft();
    invalid["clear_credentials"] = json!(true);
    invalids.push(invalid);
    invalids.push(json!({"enabled": true, "app_id": "123", "access_key_id": null, "access_key_secret": null, "identity_code": null}));
    for invalid in invalids {
        let (status, message) = save(&state, invalid).await;
        assert_eq!(status, 400, "{message}");
        assert!(!message.to_string().contains("private-"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(state.live_snapshot().await.configured);
    }
}

#[tokio::test]
async fn failed_disk_write_does_not_enable_unsaved_credentials() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(settings_path(&fixture.config_path())).unwrap();
    let state = fixture.state();
    let (status, message) = save(&state, draft()).await;
    assert_eq!(status, 500, "{message}");
    assert!(!state.live_snapshot().await.configured);
    assert_eq!(state.live_settings_snapshot().await.app_id, "");
}

mod live_support;

#[tokio::test]
async fn saving_is_rejected_during_connection_and_cleanup() {
    let fixture = Fixture::new();
    let gate = Arc::new(tokio::sync::Notify::new());
    let (source, _senders) = live_support::source(1, Some(gate.clone()));
    let mut state = live_support::state(source);
    state.live_settings = Arc::new(LiveSettingsStore::new(settings_path(
        &fixture.config_path(),
    )));
    state.connect_live().await.unwrap();
    assert_eq!(save(&state, draft()).await.0, 409);
    state.disconnect_live().await;
    assert_eq!(save(&state, draft()).await.0, 409);
    assert!(!settings_path(&fixture.config_path()).exists());
    gate.notify_one();
    state.shutdown().await;
}

#[tokio::test]
async fn foreign_origin_cannot_save_credentials() {
    let fixture = Fixture::new();
    let response = router(fixture.state())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/settings")
                .header("origin", "https://foreign.example")
                .header("content-type", "application/json")
                .body(Body::from(draft().to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert!(!settings_path(&fixture.config_path()).exists());
}

#[allow(dead_code)]
mod process_support;

#[tokio::test]
async fn panel_settings_survive_a_real_server_restart_without_environment_credentials() {
    let mut process =
        process_support::ServerProcess::start("[server]\nlisten_address='127.0.0.1:0'\n").await;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/live/settings", process.base))
        .json(&draft())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    process.restart().await;
    let saved: Value = client
        .get(format!("{}/api/live/settings", process.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(saved["app_id"], "9007199254740993");
    assert_eq!(saved["access_key_secret_configured"], true);
    assert_eq!(saved["storage_available"], true);
    assert!(!saved.to_string().contains("private-"));
    let live: Value = client
        .get(format!("{}/api/live", process.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(live["configured"], true);
    assert_eq!(live["phase"], "disconnected");
}
