use super::*;
#[test]
fn parses_unicode_names_and_both_wsl_versions() {
    let text = "  NAME STATE VERSION\r\n* Ubuntu-22.04 Running 2\r\n  我的 Ubuntu 已停止 1\r\n";
    let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let rows = parse_distros(&bytes).unwrap();
    assert_eq!(rows[1].name, "我的 Ubuntu");
    assert_eq!(rows[1].version, 1);
}
#[test]
fn rejects_unrecognised_probe_output() {
    assert!(parse_distros(b"access denied").is_err());
}
#[test]
fn validates_models_before_actions() {
    assert!(!catalog().iter().any(|item| item.id == "../../etc"));
}
#[test]
fn switching_distribution_discards_old_capabilities_and_selections() {
    let manager =
        EnvironmentManager::new(std::env::temp_dir().join("missing-environment-test.json"));
    let mut state = manager.snapshot();
    state.selected_distro = Some("first".into());
    state.backend.ready = true;
    state.models[0].selected = true;
    state.models[0].downloaded = true;
    reset_distribution(&mut state, Some("second".into()));
    assert!(!state.backend.ready);
    assert!(
        state
            .models
            .iter()
            .all(|model| !model.selected && !model.downloaded)
    );
}
// The controller runs inside WSL in production and emits POSIX backend paths.
// Native Windows Python uses WindowsPath and cannot exercise that contract.
#[cfg(unix)]
#[test]
fn bundled_python_probe_matches_native_contract() {
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scripts/voice-backend.py");
    let output = Command::new("python3")
        .arg(script)
        .arg("probe")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let backend: Backend = serde_json::from_value(value["backend"].clone()).unwrap();
    let models: Vec<Model> = serde_json::from_value(value["models"].clone()).unwrap();
    assert_eq!(backend.python_path, PYTHON);
    assert_eq!(models.len(), catalog().len());
    for (actual, expected) in models.iter().zip(catalog()) {
        assert_eq!(actual.id, expected.id);
        assert_eq!(actual.capability, expected.capability);
    }
    assert!(value["inferenceRunning"].is_boolean());
}
#[test]
fn shutdown_refuses_external_windows_installer() {
    let manager =
        EnvironmentManager::new(std::env::temp_dir().join("missing-environment-test.json"));
    manager.state.lock().unwrap().busy = true;
    *manager.operation.lock().unwrap() = "install_wsl".into();
    let error = manager.shutdown().unwrap_err();
    assert!(error.contains("Windows 安装器"));
    assert!(!manager.cancel.load(Ordering::SeqCst));
}
#[test]
fn shutdown_cancels_and_waits_for_local_detection_worker() {
    let manager = Arc::new(EnvironmentManager::new(
        std::env::temp_dir().join("missing-environment-test.json"),
    ));
    manager.state.lock().unwrap().busy = true;
    *manager.operation.lock().unwrap() = "detect".into();
    let worker = Arc::clone(&manager);
    let thread = std::thread::spawn(move || {
        let start = Instant::now();
        while !worker.cancel.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(1) {
            std::thread::sleep(Duration::from_millis(10));
        }
        worker.state.lock().unwrap().busy = false;
    });
    assert!(manager.shutdown().is_ok());
    assert!(manager.cancel.load(Ordering::SeqCst));
    assert!(!manager.snapshot().busy);
    thread.join().unwrap();
}
