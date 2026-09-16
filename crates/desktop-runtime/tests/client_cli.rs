use std::process::Command;

#[test]
fn help_documents_explicit_simulation_without_connecting() {
    let output = Command::new(env!("CARGO_BIN_EXE_meowlive-client"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("--config"));
    assert!(text.contains("--simulate"));
    assert!(text.contains("--once"));
}

#[test]
fn missing_explicit_configuration_exits_with_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_meowlive-client"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("--config")
    );
}
