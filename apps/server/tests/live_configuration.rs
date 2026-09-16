use meowlive_server::{
    bootstrap::build_live_source,
    config::{AppConfig, LiveConfig},
};

#[test]
fn disabled_source_does_not_require_credentials_or_create_a_platform_client() {
    assert!(build_live_source(&LiveConfig::default()).unwrap().is_none());
}

#[test]
fn missing_environment_keeps_other_server_features_available() {
    let config = LiveConfig {
        enabled: true,
        app_id: 1,
        access_key_id_env: format!("MEOWLIVE_ABSENT_{}", uuid::Uuid::new_v4().simple()),
        ..LiveConfig::default()
    };
    assert!(build_live_source(&config).unwrap().is_none());
}

#[test]
fn rejects_plaintext_secrets_invalid_environment_names_and_inverted_backoff() {
    for text in [
        "[live]\naccess_key_secret='secret'",
        "[live]\naccess_key_id_env='A=B'",
        "[live]\nidentity_code_env=''",
        "[live]\nreconnect_initial_ms=1000\nreconnect_max_ms=10",
        "[live]\napp_id=18446744073709551615\nenabled=true",
    ] {
        assert!(AppConfig::parse(text).is_err(), "accepted {text}");
    }
}
