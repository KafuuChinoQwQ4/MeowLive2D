use meowlive_desktop_runtime::config::ClientConfig;

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
