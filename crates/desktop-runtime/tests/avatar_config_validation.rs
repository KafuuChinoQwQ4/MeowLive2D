use meowlive_desktop_runtime::avatar::VtsConfig;

#[test]
fn defaults_are_opt_in_and_unknown_keys_fail() {
    let config: VtsConfig = toml::from_str("").unwrap();
    assert!(!config.enabled);
    assert!(config.validate().is_ok());
    assert!(toml::from_str::<VtsConfig>("enable = true").is_err());
}

#[test]
fn rejects_unsafe_urls_parameters_and_unbounded_deadlines() {
    for url in [
        "http://127.0.0.1:8001",
        "wss://localhost:8001",
        "ws://user:secret@localhost:8001",
        "ws://localhost:8001/?token=secret",
        "ws://localhost:8001/#secret",
    ] {
        assert!(
            VtsConfig {
                websocket_url: url.into(),
                ..VtsConfig::default()
            }
            .validate()
            .is_err(),
            "{url}"
        );
    }
    for parameter in [
        "",
        "abc",
        "bad_name",
        "嘴巴参数",
        "abcdefghijklmnopqrstuvwxyz1234567",
    ] {
        assert!(
            VtsConfig {
                mouth_parameter: parameter.into(),
                ..VtsConfig::default()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        VtsConfig {
            request_timeout_ms: 0,
            ..VtsConfig::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        VtsConfig {
            auth_timeout_ms: u64::MAX,
            ..VtsConfig::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        VtsConfig {
            reconnect_delay_ms: 0,
            ..VtsConfig::default()
        }
        .validate()
        .is_err()
    );
}
