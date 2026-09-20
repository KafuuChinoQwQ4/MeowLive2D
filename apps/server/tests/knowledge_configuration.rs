use meowlive_server::config::AppConfig;
#[test]
fn memory_requires_viewer_storage_and_independent_endpoint() {
    assert!(
        AppConfig::parse(
            "[viewers]\nenabled=false\n[memory]\nenabled=true\nendpoint='http://127.0.0.1:8000/v1'\nmodel='extract'"
        )
        .is_err()
    );
    assert!(
        AppConfig::parse(
            "[auth]\nenabled=true\n[viewers]\nenabled=true\n[memory]\nenabled=true\nmodel='extract'"
        )
        .is_err()
    );
    assert!(AppConfig::parse("[auth]\nenabled=true\n[viewers]\nenabled=true\n[memory]\nenabled=true\nendpoint='http://127.0.0.1:8000/v1'\nmodel='extract'").is_ok());
}
#[test]
fn graph_requires_viewer_storage() {
    assert!(AppConfig::parse("[viewers]\nenabled=false\n[graph]\nenabled=true").is_err());
    assert!(
        AppConfig::parse("[auth]\nenabled=true\n[viewers]\nenabled=true\n[graph]\nenabled=true")
            .is_ok()
    );
}
