use meowlive_server::{bootstrap::build_model, config::LlmConfig};

#[test]
fn missing_configuration_and_missing_credentials_do_not_enable_agent() {
    assert!(build_model(&LlmConfig::default()).unwrap().is_none());
    let config = LlmConfig {
        base_url: "https://example.test/v1".into(),
        model: "test-model".into(),
        api_key_env: format!("MEOWLIVE_TEST_ABSENT_{}", uuid::Uuid::new_v4().simple()),
        ..LlmConfig::default()
    };
    let error = match build_model(&config) {
        Err(error) => error,
        _ => panic!("missing credentials must fail before startup"),
    };
    assert!(error.contains("环境变量"));
    assert!(!error.contains("example.test"));
}

#[test]
fn explicitly_keyless_local_provider_can_be_built_without_environment_changes() {
    let config = LlmConfig {
        base_url: "http://127.0.0.1:9000/v1".into(),
        model: "local-model".into(),
        api_key_env: String::new(),
        ..LlmConfig::default()
    };
    assert!(build_model(&config).unwrap().is_some());
}
