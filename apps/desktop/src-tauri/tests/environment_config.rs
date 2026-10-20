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
            engine_root: "/opt/meowlive-voice/engine".into(),
            python_path: "/opt/meowlive-voice/venv/bin/python".into(),
            model_root: "/opt/meowlive-voice/models/gpt-sovits-v2proplus".into(),
            ..Default::default()
        },
        models: vec![Model {
            id: "gpt-sovits-v2proplus".into(),
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

#[test]
fn applies_existing_paths_and_keeps_downloaded_asr_in_managed_storage() {
    let mut state = ready();
    state.backend.engine_root = "/custom/engine".into();
    state.backend.python_path = "/custom/env/bin/python".into();
    state.backend.model_root = "/custom/engine".into();
    state.models.push(Model {
        id: "whisper".into(),
        name: "Whisper".into(),
        capability: "transcription".into(),
        downloaded: true,
        selected: true,
    });
    let original = "[training]\npython = '/keep/python'\nengine_root = '/keep/engine'";
    let value: toml_edit::DocumentMut = configure(original, &state).unwrap().parse().unwrap();
    assert_eq!(
        value["training"]["wsl_python"].as_str(),
        Some("/custom/env/bin/python")
    );
    assert_eq!(
        value["training"]["wsl_engine_root"].as_str(),
        Some("/custom/engine")
    );
    assert_eq!(value["training"]["python"].as_str(), Some("/keep/python"));
    assert_eq!(
        value["training"]["engine_root"].as_str(),
        Some("/keep/engine")
    );
    assert_eq!(
        value["training"]["default_sovits_weights"].as_str(),
        Some("/custom/engine/GPT_SoVITS/pretrained_models/v2Pro/s2Gv2ProPlus.pth")
    );
    assert_eq!(
        value["training"]["asr_model"].as_str(),
        Some("/opt/meowlive-voice/models/whisper")
    );
}

#[test]
fn every_selected_gpt_version_configures_its_actual_pair() {
    for (id, gpt, sovits) in [
        (
            "gpt-sovits-v2",
            "s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt",
            "s2G2333k.pth",
        ),
        ("gpt-sovits-v2pro", "s1v3.ckpt", "s2Gv2Pro.pth"),
        ("gpt-sovits-v2proplus", "s1v3.ckpt", "s2Gv2ProPlus.pth"),
        ("gpt-sovits-v3", "s1v3.ckpt", "s2Gv3.pth"),
        ("gpt-sovits-v4", "s1v3.ckpt", "s2Gv4.pth"),
    ] {
        let mut environment = ready();
        environment.models[0].id = id.into();
        environment.backend.model_root = format!("/opt/meowlive-voice/models/{id}");
        let configured = configure("", &environment).unwrap();
        let doc: toml_edit::DocumentMut = configured.parse().unwrap();
        assert!(
            doc["training"]["default_gpt_weights"]
                .as_str()
                .unwrap()
                .ends_with(gpt)
        );
        assert!(
            doc["training"]["default_sovits_weights"]
                .as_str()
                .unwrap()
                .ends_with(sovits)
        );
    }
}
