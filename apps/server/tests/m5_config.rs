use meowlive_server::config::AppConfig;

#[test]
fn local_mode_requires_literal_loopback_without_secrets_and_small_budget() {
    let valid = "[llm]\nmode='local'\nbase_url='http://127.0.0.1:8080/v1'\nmodel='local-quantized'\napi_key_env=''\nmax_tokens=512\nmax_retries=0";
    assert!(AppConfig::parse(valid).is_ok());
    for bad in [
        valid.replace("127.0.0.1", "example.com"),
        valid.replace("api_key_env=''", "api_key_env='PRIVATE_KEY'"),
        valid.replace("max_tokens=512", "max_tokens=4096"),
        valid.replace("max_retries=0", "max_retries=1"),
        valid.replace("mode='local'", "mode='unknown'"),
    ] {
        assert!(
            AppConfig::parse(&bad).is_err(),
            "accepted invalid local preset"
        );
    }
}

#[test]
fn training_requires_explicit_paths_and_bounded_timeout() {
    assert!(AppConfig::parse("").is_ok());
    assert!(AppConfig::parse("[training]\nenabled=true").is_err());
    let config = "[training]\nenabled=true\npython='/test/python'\nengine_root='/test/GPT-SoVITS'\ntimeout_seconds=3600";
    assert!(AppConfig::parse(config).is_ok());
    assert!(AppConfig::parse(&config.replace("3600", "0")).is_err());
    assert!(AppConfig::parse(&config.replace("/test/python", "relative/python")).is_err());
}

#[test]
fn transcription_model_can_use_engine_default_or_explicit_absolute_path() {
    let default = AppConfig::parse("[training]\nasr_model='' ").unwrap();
    assert!(default.training.asr_model.as_os_str().is_empty());
    assert!(AppConfig::parse("[training]\nasr_model='/local/whisper'").is_ok());
    assert!(AppConfig::parse("[training]\nasr_model='relative/whisper'").is_err());
}
