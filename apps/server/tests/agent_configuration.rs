use meowlive_server::config::AppConfig;

#[test]
fn old_configuration_keeps_agent_paused_and_llm_unconfigured() {
    let config = AppConfig::parse("").unwrap();
    assert!(config.llm.base_url.is_empty());
    assert!(config.llm.model.is_empty());
    assert!(!config.agent.proactive_enabled);
    assert_eq!(config.agent.cooldown_ms, 30_000);
    assert_eq!(config.agent.limits().pending_capacity, 128);
}

#[test]
fn reads_persona_scheduler_limits_and_private_model_configuration() {
    let config = AppConfig::parse(
        r#"
[agent]
persona = "温柔的猫咪主播"
topic = "游戏"
proactive_enabled = true
cooldown_ms = 12000
pending_capacity = 64
[llm]
base_url = "http://127.0.0.1:9000/v1"
model = "test-model"
api_key_env = "MEOWLIVE_LLM_API_KEY"
json_mode = false
timeout_seconds = 10
max_response_bytes = 65536
max_tokens = 512
max_retries = 1
"#,
    )
    .unwrap();
    assert_eq!(config.agent.settings().persona, "温柔的猫咪主播");
    assert_eq!(config.agent.limits().pending_capacity, 64);
    assert!(config.llm.is_configured());
    assert!(!config.llm.json_mode);
}

#[test]
fn rejects_unknown_fields_and_invalid_resource_bounds_before_startup() {
    for invalid in [
        "[agent]\ncooldown_ms=0",
        "[agent]\npersona=''",
        "[agent]\npending_capacity=0",
        "[agent]\nbatch_size=17",
        "[llm]\napi_key='secret'",
        "[llm]\nmax_retries=20",
        "[llm]\ntimeout_seconds=0",
        "[llm]\nmax_response_bytes=0",
        "[llm]\nbase_url='file:///etc/passwd'\nmodel='test'",
        "[llm]\nbase_url='http://user:password@localhost'\nmodel='test'",
        "[llm]\napi_key_env='NOT A VARIABLE'",
        "[llm]\nbase_url='https://example.test/v1'\nmodel=''",
    ] {
        assert!(AppConfig::parse(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn configuration_errors_do_not_echo_accidentally_pasted_keys() {
    let error = AppConfig::parse("[llm]\napi_key='private-api-key-value'").unwrap_err();
    assert!(!error.contains("private-api-key-value"));
}
