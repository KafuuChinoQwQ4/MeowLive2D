use meowlive_desktop_runtime::config::ClientConfig;

#[test]
fn old_configuration_keeps_vts_disabled_and_lip_sync_has_safe_defaults() {
    let config = ClientConfig::from_toml("server_url='http://127.0.0.1:19600'").unwrap();
    assert!(!config.vtube_studio.enabled);
    assert_eq!(config.vtube_studio.mouth_parameter, "MeowMouthOpen");
    assert_eq!(config.lip_sync.update_hz, 30);
}

#[test]
fn reads_explicit_avatar_and_lip_sync_settings() {
    let config = ClientConfig::from_toml("[vtube_studio]\nenabled=true\nwebsocket_url='ws://127.0.0.1:8002'\n[lip_sync]\ngain=3.0\nrelease_ms=120\n").unwrap();
    assert!(config.vtube_studio.enabled);
    assert_eq!(config.vtube_studio.websocket_url, "ws://127.0.0.1:8002");
    assert_eq!(config.lip_sync.gain, 3.0);
}

#[test]
fn rejects_invalid_settings_even_when_the_feature_is_disabled() {
    for input in [
        "[lip_sync]\ngain=nan",
        "[lip_sync]\nnoise_floor=0.8",
        "[lip_sync]\nupdate_hz=60",
        "[vtube_studio]\nwebsocket_url='file:///tmp/vts'",
        "[vtube_studio]\nmouth_parameter='invalid name'",
        "[vtube_studio]\nauth_timeout_ms=0",
    ] {
        assert!(ClientConfig::from_toml(input).is_err(), "accepted {input}");
    }
}

#[test]
fn token_file_path_resolves_against_config_directory() {
    let mut config = ClientConfig::default();
    config.resolve_paths(std::path::Path::new("config/machine/client.toml"));
    assert_eq!(
        config.vtube_studio.token_path,
        std::path::Path::new("config/machine/local/vts-token.json")
    );
}
