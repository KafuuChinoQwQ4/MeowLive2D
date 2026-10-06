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
fn environment_cache_restores_probe_results_without_transient_runtime_state() {
    let directory =
        std::env::temp_dir().join(format!("meowlive-environment-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let path = directory.join("environment.json");
    let manager = EnvironmentManager::new(path.clone());
    {
        let mut state = manager.state.lock().unwrap();
        state.distros = vec![Distro {
            name: "Ubuntu".into(),
            version: 2,
        }];
        state.selected_distro = Some("Ubuntu".into());
        state.backend.ready = true;
        state.backend.python_path = "/opt/venv/bin/python".into();
        state.models[0].downloaded = true;
        state.models[0].selected = true;
        state.inference_running = true;
        state.logs.push("transient log".into());
    }
    manager.save().unwrap();

    let restored = EnvironmentManager::new(path.clone());
    let state = restored.snapshot();
    assert!(restored.cache_loaded);
    assert_eq!(state.selected_distro.as_deref(), Some("Ubuntu"));
    assert!(state.backend.ready);
    assert_eq!(state.backend.python_path, "/opt/venv/bin/python");
    assert!(state.models[0].downloaded && state.models[0].selected);
    assert!(!state.inference_running && !state.busy);
    assert!(state.logs.is_empty());
    let _ = std::fs::remove_dir_all(directory);
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
        .args(["-c", "import importlib.util,json,sys; s=importlib.util.spec_from_file_location('backend',sys.argv[1]); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); print(json.dumps(m.probe()))"])
        .arg(script)
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
    assert!(backend.python_path.starts_with('/'));
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

#[test]
fn existing_config_uses_only_training_paths_and_rejects_invalid_paths() {
    let parsed = existing_training_paths("[llm]\napi_key = 'private'\n[training]\npython = '/env with space/python'\nengine_root = '/custom/engine'").unwrap();
    assert_eq!(
        parsed,
        ("/env with space/python".into(), "/custom/engine".into())
    );
    assert!(
        existing_training_paths("[training]\npython = 'relative'\nengine_root = '/engine'")
            .is_err()
    );
    assert!(existing_training_paths("[training]\npython = '/python'").is_err());
}

#[cfg(unix)]
#[test]
fn config_reads_do_not_expose_content_in_environment_logs() {
    let manager =
        EnvironmentManager::new(std::env::temp_dir().join("missing-environment-test.json"));
    let mut cmd = Command::new("python3");
    cmd.args([
        "-c",
        "print('[llm]\\napi_key = \"private-config-value\"')",
        "read-config",
    ]);
    let output = manager.run(cmd, None, Duration::from_secs(5)).unwrap();
    assert!(decode(&output).contains("private-config-value"));
    assert!(manager.snapshot().logs.is_empty());
}

#[cfg(unix)]
#[test]
fn utf16_command_logs_keep_line_boundaries_and_unicode_names() {
    let manager = EnvironmentManager::new(std::env::temp_dir().join("missing-log-test.json"));
    let mut cmd = Command::new("python3");
    cmd.args(["-c", "import sys; sys.stdout.buffer.write('  NAME STATE VERSION\\r\\n* archlinux Running 2\\r\\n  我的Ubuntu Stopped 2\\r\\n'.encode('utf-16le'))"]);
    let bytes = manager.run(cmd, None, Duration::from_secs(5)).unwrap();
    assert_eq!(parse_distros(&bytes).unwrap()[1].name, "我的Ubuntu");
    let logs = manager.snapshot().logs;
    assert_eq!(
        logs,
        vec![
            "  NAME STATE VERSION",
            "* archlinux Running 2",
            "  我的Ubuntu Stopped 2"
        ]
    );
}
