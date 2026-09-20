use meowlive_desktop_runtime::config::ClientConfig;
use meowlive_desktop_runtime::obs;
use meowlive_protocol::obs::ObsSettingsRequest;

fn isolated_config(name: &str) -> (ClientConfig, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("meowlive-obs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut config = ClientConfig::default();
    config.obs.password_env = "MEOWLIVE_OBS_SETTINGS_TEST_UNSET".into();
    config.resolve_paths(&root.join("desktop.toml"));
    (config, root)
}

fn update(password: Option<&str>, clear_password: bool) -> ObsSettingsRequest {
    ObsSettingsRequest {
        enabled: true,
        websocket_url: "ws://127.0.0.1:4455".into(),
        password: password.map(str::to_owned),
        clear_password,
    }
}

#[tokio::test]
async fn obs_settings_persist_reload_retain_and_explicitly_clear_credentials() {
    let (config, root) = isolated_config("persist");
    let initial = obs::settings(&config.obs).await.unwrap();
    assert!(initial.storage_available);
    assert!(!initial.enabled && !initial.password_configured);
    let saved = obs::save_settings(&config.obs, update(Some("private-test-password"), false))
        .await
        .unwrap();
    assert!(saved.enabled && saved.password_configured);
    assert!(
        !serde_json::to_string(&saved)
            .unwrap()
            .contains("private-test-password")
    );
    let mut reloaded = ClientConfig::default();
    reloaded.resolve_paths(&root.join("desktop.toml"));
    assert_eq!(obs::settings(&reloaded.obs).await.unwrap(), saved);
    assert!(
        obs::save_settings(&reloaded.obs, update(None, false))
            .await
            .unwrap()
            .password_configured
    );
    assert!(
        !obs::save_settings(&reloaded.obs, update(None, true))
            .await
            .unwrap()
            .password_configured
    );
    assert!(
        !obs::settings(&config.obs)
            .await
            .unwrap()
            .password_configured
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(root.join("desktop.obs.local.json")).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o077, 0);
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn obs_invalid_updates_and_endpoint_changes_do_not_overwrite_saved_credentials() {
    let (config, root) = isolated_config("invalid");
    let saved = obs::save_settings(&config.obs, update(Some("private-test-password"), false))
        .await
        .unwrap();
    for url in [
        "ws://remote.example:4455",
        "ws://127.0.0.1:4456",
        "ws://user:private-test-password@127.0.0.1:4455",
    ] {
        let mut request = update(None, false);
        request.websocket_url = url.into();
        let error = obs::save_settings(&config.obs, request).await.unwrap_err();
        assert!(!error.contains("private-test-password"));
        assert_eq!(obs::settings(&config.obs).await.unwrap(), saved);
    }
    assert!(
        obs::save_settings(&config.obs, update(Some("ambiguous"), true))
            .await
            .is_err()
    );
    let mut request = update(None, true);
    request.websocket_url = "ws://127.0.0.1:4456".into();
    let saved = obs::save_settings(&config.obs, request).await.unwrap();
    assert_eq!(saved.websocket_url, "ws://127.0.0.1:4456");
    assert!(!saved.password_configured);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn obs_settings_without_a_config_location_report_unavailable_storage() {
    let config = ClientConfig::default();
    assert!(!obs::settings(&config.obs).await.unwrap().storage_available);
    assert!(
        obs::save_settings(&config.obs, update(None, false))
            .await
            .is_err()
    );
}

#[test]
fn obs_configuration_accepts_explicit_loopback_settings() {
    let parsed = ClientConfig::from_toml(
        r#"
[obs]
enabled = true
websocket_url = "ws://127.0.0.1:4455"
password_env = "MEOWLIVE_OBS_PASSWORD"
timeout_ms = 5000
"#,
    );
    assert!(
        parsed.is_ok(),
        "OBS local configuration rejected: {parsed:?}"
    );
}

#[test]
fn obs_configuration_rejects_remote_credentialed_or_unbounded_settings() {
    for url in [
        "ws://example.com:4455",
        "ws://localhost:4455",
        "ws://192.168.1.2:4455",
        "ws://user:secret@127.0.0.1:4455",
        "ws://127.0.0.1:4455/obs",
        "ws://127.0.0.1:4455/?secret=yes",
        "wss://127.0.0.1:4455",
        "ws://127.0.0.1:4455/#fragment",
    ] {
        assert!(ClientConfig::from_toml(&format!("[obs]\nwebsocket_url = {url:?}")).is_err());
    }
    for timeout in [0, 99, 10_001] {
        assert!(ClientConfig::from_toml(&format!("[obs]\ntimeout_ms = {timeout}")).is_err());
    }
    for env in ["", "KEY=SECRET", "KEY WITH SPACE"] {
        assert!(ClientConfig::from_toml(&format!("[obs]\npassword_env = {env:?}")).is_err());
    }
}

#[test]
fn obs_configuration_checks_the_original_url_before_parser_normalization() {
    for url in [
        "ws://@127.0.0.1:4455",
        "ws://127.0.0.1:4455/private/..",
        "ws://127.1:4455",
        "ws://2130706433:4455",
        "ws://0x7f000001:4455",
        "ws://127.0.0.1:4455/./",
        "ws://127.0.0.1:4455\\",
    ] {
        assert!(
            ClientConfig::from_toml(&format!("[obs]\nwebsocket_url = {url:?}")).is_err(),
            "non-literal or forbidden URL accepted: {url}"
        );
    }
    for url in ["ws://127.0.0.1:4455", "ws://127.0.0.2/", "ws://[::1]:4455/"] {
        assert!(ClientConfig::from_toml(&format!("[obs]\nwebsocket_url = {url:?}")).is_ok());
    }
}
