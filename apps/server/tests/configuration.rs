use meowlive_server::config::AppConfig;

#[test]
fn checked_in_example_is_a_valid_starting_configuration() {
    let config = AppConfig::parse(include_str!("../../../config/server.example.toml")).unwrap();
    assert_eq!(config.server.listen_address, "127.0.0.1:19600");
    assert_eq!(config.speech.max_concurrency, 1);
    assert!(config.viewers.enabled);
    assert!(!config.auth.enabled);
}

#[test]
fn rejects_unbounded_or_invalid_runtime_settings() {
    for text in [
        "[server]\nqueue_capacity=0",
        "[speech]\nmax_concurrency=2",
        "[speech]\ntimeout_seconds=0",
        "[server]\nlisten_address='nonsense'",
    ] {
        assert!(AppConfig::parse(text).is_err(), "accepted {text}");
    }
}

#[test]
fn rejects_misspelled_settings() {
    assert!(AppConfig::parse("[speech]\nreferenc_audio='x'").is_err());
}
