use meowlive_desktop_runtime::{audio::DeviceBackend, config::ClientConfig, connection::audio_url};

#[test]
fn config_reads_explicit_toml_and_rejects_unsafe_resource_bounds() {
    let config =
        ClientConfig::from_toml("server_url='http://localhost:8080'\nmax_buffer_samples=48000\n")
            .unwrap();
    assert_eq!(config.max_buffer_samples, 48000);
    assert!(ClientConfig::from_toml("server_url='file:///tmp/a'").is_err());
    assert!(
        ClientConfig::from_toml("server_url='http://localhost:8080'\nmax_buffer_samples=0")
            .is_err()
    );
}

#[test]
fn audio_connection_uses_escaped_pairing_ids() {
    let url = audio_url("http://localhost:8080", "session a", "bridge&b").unwrap();
    assert_eq!(
        url,
        "ws://localhost:8080/ws/audio?session_id=session+a&bridge_id=bridge%26b"
    );
}

#[cfg(not(windows))]
#[test]
fn non_windows_device_is_explicitly_unavailable() {
    assert!(DeviceBackend::new(48_000).is_err());
}
