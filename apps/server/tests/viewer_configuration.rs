use meowlive_server::config::AppConfig;

#[test]
fn persistent_viewers_default_to_owner_access_and_require_stable_scope() {
    let defaults = AppConfig::parse("").unwrap();
    assert!(defaults.viewers.enabled);
    assert!(!defaults.auth.enabled);
    assert!(AppConfig::parse("[viewers]\nenabled=true").is_ok());
    assert!(
        AppConfig::parse("[viewers]\nenabled=true\nscope_id=''\n[auth]\nenabled=true").is_err()
    );
    let config =
        AppConfig::parse("[viewers]\nenabled=true\nscope_id='my-avatar'\n[auth]\nenabled=true")
            .unwrap();
    assert_eq!(config.viewers.scope_id, "my-avatar");
    assert!(AppConfig::parse("[viewers]\ndatabase_url_env='BAD NAME'").is_err());
    assert!(AppConfig::parse("[viewers]\nscope_id='a b'").is_err());
    assert!(AppConfig::parse("[viewers]\ndatabase_url='postgres://secret'").is_err());
}
