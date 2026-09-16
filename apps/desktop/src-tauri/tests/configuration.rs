use meowlive_desktop::startup::{load_configuration, parse_arguments};
use std::path::PathBuf;

#[test]
fn explicit_configuration_is_resolved_against_its_own_directory() {
    let directory = std::env::temp_dir().join(format!("meowlive-shell-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("desktop.toml");
    std::fs::write(&path, "server_url='ws://localhost:19601'\nmodel_directory='models'\n[vtube_studio]\ntoken_path='private/token.json'").unwrap();
    let loaded = load_configuration(Some(&path), &directory.join("unused")).unwrap();
    assert_eq!(loaded.server_url, "http://localhost:19601");
    assert_eq!(
        loaded.config.model_directory,
        Some(directory.join("models"))
    );
    assert_eq!(
        loaded.config.vtube_studio.token_path,
        directory.join("private/token.json")
    );
    assert!(!directory.join("unused").exists());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn first_run_creates_editable_config_and_preserves_user_changes() {
    let directory = std::env::temp_dir().join(format!("meowlive-first-run-{}", std::process::id()));
    let first = load_configuration(None, &directory).unwrap();
    assert!(first.config_path.is_absolute());
    std::fs::write(&first.config_path, "server_url='http://localhost:19602'").unwrap();
    let second = load_configuration(None, &directory).unwrap();
    assert_eq!(second.server_url, "http://localhost:19602");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn arguments_require_values_and_reject_unknown_or_duplicate_options() {
    let options =
        parse_arguments(["--config", "custom.toml", "--simulate"].map(String::from)).unwrap();
    assert_eq!(options.config_path, Some(PathBuf::from("custom.toml")));
    assert!(options.simulation);
    assert!(parse_arguments(["--config"].map(String::from)).is_err());
    assert!(parse_arguments(["--config", "--simulate"].map(String::from)).is_err());
    assert!(parse_arguments(["--once"].map(String::from)).is_err());
    assert!(parse_arguments(["--config", "a", "--config", "b"].map(String::from)).is_err());
}
