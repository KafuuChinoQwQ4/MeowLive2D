use meowlive_desktop::environment_config::configure;
use meowlive_protocol::desktop_environment::{Backend, EnvironmentSnapshot, Model};
fn ready() -> EnvironmentSnapshot {
    EnvironmentSnapshot {
        phase: "idle".into(),
        busy: false,
        message: String::new(),
        logs: vec![],
        distros: vec![],
        selected_distro: Some("Ubuntu".into()),
        backend: Backend {
            ready: true,
            gpu: true,
            ..Default::default()
        },
        models: vec![Model {
            id: "gpt-sovits-v2".into(),
            name: "v2".into(),
            capability: "training_inference".into(),
            downloaded: true,
            selected: true,
        }],
        progress: 100,
        inference_running: false,
    }
}
#[test]
fn retains_user_configuration_and_connects_only_selected_ready_models() {
    let original = "# private configuration\n[llm]\nmodel = 'existing'\n[viewers]\nenabled = true\n[training]\ndirectory = 'custom-training'\n";
    let configured = configure(original, &ready()).unwrap();
    assert!(configured.contains("# private configuration"));
    let value: toml_edit::DocumentMut = configured.parse().unwrap();
    assert_eq!(value["llm"]["model"].as_str(), Some("existing"));
    assert_eq!(
        value["training"]["directory"].as_str(),
        Some("custom-training")
    );
    assert_eq!(
        value["training"]["wsl_distribution"].as_str(),
        Some("Ubuntu")
    );
    assert_eq!(value["training"]["managed_inference"].as_bool(), Some(true));
    assert_eq!(
        value["speech"]["base_url"].as_str(),
        Some("http://127.0.0.1:9880")
    );
}
#[test]
fn refuses_incomplete_environment_and_clears_previous_asr_selection() {
    let mut state = ready();
    state.models[0].downloaded = false;
    assert!(configure("", &state).is_err());
    state.models[0].downloaded = true;
    let configured = configure("[training]\nasr_model = '/old/asr'", &state).unwrap();
    let value: toml_edit::DocumentMut = configured.parse().unwrap();
    assert_eq!(value["training"]["asr_model"].as_str(), Some(""));
    state.busy = true;
    assert!(configure("", &state).is_err());
}

#[test]
fn cpu_backend_enables_inference_without_claiming_training_ready() {
    let mut state = ready();
    state.backend.gpu = false;
    let value: toml_edit::DocumentMut = configure("", &state).unwrap().parse().unwrap();
    assert_eq!(value["training"]["enabled"].as_bool(), Some(false));
    assert_eq!(value["training"]["managed_inference"].as_bool(), Some(true));
}
