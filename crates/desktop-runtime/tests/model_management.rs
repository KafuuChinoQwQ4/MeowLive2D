use meowlive_protocol::resources::{DesktopResourceOperation, DesktopResourceResult};
use serde_json::json;

#[test]
fn installed_model_contract_uses_ids_and_rejects_remote_paths() {
    for value in [
        json!({"type":"list_imported_models"}),
        json!({"type":"delete_imported_model","id":"opaque-install-id"}),
    ] {
        let operation = serde_json::from_value::<DesktopResourceOperation>(value.clone())
            .expect("installed models can be listed and deleted by ID");
        assert_eq!(serde_json::to_value(operation).unwrap(), value);
    }
    assert!(
        serde_json::from_value::<DesktopResourceOperation>(json!({
            "type":"delete_imported_model","id":"opaque-install-id","path":"C:/outside"
        }))
        .is_err()
    );
    let value = json!({"type":"model_deleted","id":"opaque-install-id","restart_required":true});
    let result = serde_json::from_value::<DesktopResourceResult>(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), value);
}

use meowlive_desktop_runtime::assets::{
    AssetError, ModelImportLimits, delete_imported_model, install_model_package,
    list_imported_models, validate_model_package,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/model-management-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(path.join("Live2DModels")).unwrap();
        Self(path)
    }
    fn models(&self) -> PathBuf {
        self.0.join("Live2DModels")
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn package(root: &Path, model_id: Option<&str>) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("model.moc3"), b"moc3").unwrap();
    fs::write(root.join("texture.png"), b"png").unwrap();
    fs::write(
        root.join("model.model3.json"),
        json!({
            "Version":3,"FileReferences":{"Moc":"model.moc3","Textures":["texture.png"]}
        })
        .to_string(),
    )
    .unwrap();
    if let Some(model_id) = model_id {
        fs::write(
            root.join("model.vtube.json"),
            json!({
                "ModelID":model_id,"FileReferences":{"Model":"model.model3.json"}
            })
            .to_string(),
        )
        .unwrap();
    }
}

#[test]
fn legacy_install_listing_preserves_vts_identity_and_skips_unrelated_or_corrupt_dirs() {
    let sandbox = Sandbox::new();
    package(&sandbox.models().join("魔女"), Some("vts-model-1"));
    fs::create_dir_all(sandbox.models().join("unrelated")).unwrap();
    package(&sandbox.models().join("broken"), Some("bad-model"));
    fs::write(
        sandbox.models().join("broken/model.model3.json"),
        b"corrupt",
    )
    .unwrap();
    package(&sandbox.models().join(".meowlive-import-123-0"), None);
    let listed = list_imported_models(&sandbox.models()).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "魔女");
    assert_eq!(listed[0].model_id.as_deref(), Some("vts-model-1"));
    assert_eq!(listed[0].id.len(), 64);
    assert!(!listed[0].id.contains("魔女"));
}

#[test]
fn deletion_removes_only_installed_copy_and_rejects_repeat_and_path_ids() {
    let sandbox = Sandbox::new();
    let source = sandbox.0.join("source");
    package(&source, Some("vts-model-1"));
    package(&sandbox.models().join("other"), Some("other-model"));
    let validated = validate_model_package(&source, &ModelImportLimits::default()).unwrap();
    install_model_package(&validated, &sandbox.models(), "installed").unwrap();
    let selected = list_imported_models(&sandbox.models())
        .unwrap()
        .into_iter()
        .find(|model| model.name == "installed")
        .unwrap();
    for id in ["../source", "/tmp", "C:\\outside", "", "."] {
        assert!(delete_imported_model(&sandbox.models(), id, None).is_err());
        assert!(sandbox.models().join("installed/model.moc3").is_file());
    }
    delete_imported_model(&sandbox.models(), &selected.id, None).unwrap();
    assert!(!sandbox.models().join("installed").exists());
    assert!(source.join("model.moc3").is_file());
    assert!(sandbox.models().join("other/model.moc3").is_file());
    assert!(delete_imported_model(&sandbox.models(), &selected.id, None).is_err());
}

#[test]
fn deletion_revalidates_corrupt_or_missing_manifest_before_removing_any_file() {
    let sandbox = Sandbox::new();
    let model = sandbox.models().join("installed");
    package(&model, Some("vts-model-1"));
    let selected = list_imported_models(&sandbox.models()).unwrap().remove(0);
    fs::write(model.join("model.model3.json"), b"corrupt").unwrap();
    assert!(delete_imported_model(&sandbox.models(), &selected.id, None).is_err());
    assert!(model.join("model.moc3").is_file());
    fs::remove_file(model.join("model.model3.json")).unwrap();
    assert!(delete_imported_model(&sandbox.models(), &selected.id, None).is_err());
    assert!(model.join("texture.png").is_file());
}

#[cfg(unix)]
#[test]
fn deletion_rejects_nested_symlinks_and_replaced_install_root() {
    use std::os::unix::fs::symlink;
    let sandbox = Sandbox::new();
    let model = sandbox.models().join("installed");
    package(&model, Some("vts-model-1"));
    let outside = sandbox.0.join("original");
    package(&outside, Some("outside-model"));
    let selected = list_imported_models(&sandbox.models()).unwrap().remove(0);
    symlink(&outside, model.join("linked")).unwrap();
    assert_eq!(
        delete_imported_model(&sandbox.models(), &selected.id, None).unwrap_err(),
        AssetError::SymlinkNotAllowed
    );
    assert!(model.join("model.moc3").is_file());
    assert!(outside.join("model.moc3").is_file());
    fs::remove_file(model.join("linked")).unwrap();
    fs::remove_dir_all(&model).unwrap();
    symlink(&outside, &model).unwrap();
    assert!(delete_imported_model(&sandbox.models(), &selected.id, None).is_err());
    assert!(outside.join("model.moc3").is_file());
    assert!(list_imported_models(&sandbox.models()).unwrap().is_empty());
}

#[test]
fn interrupted_delete_stays_listed_and_can_finish_without_its_manifest() {
    let sandbox = Sandbox::new();
    let model = sandbox.models().join("installed");
    package(&model, Some("vts-model-1"));
    let selected = list_imported_models(&sandbox.models()).unwrap().remove(0);
    let receipt = sandbox
        .models()
        .join(format!(".meowlive-delete-{}.json", selected.id));
    // Simulate a process stopping after recording deletion intent and removing
    // the manifest, but before it can remove the remaining binary/texture files.
    fs::write(
        &receipt,
        json!({"name":"installed", "model_file":"model.model3.json",
        "model_id":"vts-model-1"})
        .to_string(),
    )
    .unwrap();
    fs::remove_file(model.join("model.model3.json")).unwrap();
    let pending = list_imported_models(&sandbox.models()).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].model_id.as_deref(), Some("vts-model-1"));
    delete_imported_model(&sandbox.models(), &selected.id, None).unwrap();
    assert!(!model.exists());
    assert!(!receipt.exists());
}

#[test]
fn loaded_vts_model_is_preserved_but_another_known_model_can_be_deleted() {
    let sandbox = Sandbox::new();
    let model = sandbox.models().join("installed");
    package(&model, Some("vts-model-1"));
    let selected = list_imported_models(&sandbox.models()).unwrap().remove(0);
    assert!(delete_imported_model(&sandbox.models(), &selected.id, Some("vts-model-1")).is_err());
    assert!(model.join("model.moc3").is_file());
    delete_imported_model(
        &sandbox.models(),
        &selected.id,
        Some("different-loaded-model"),
    )
    .unwrap();
    assert!(!model.exists());
}

#[test]
fn unknown_vts_identity_requires_no_loaded_model() {
    let sandbox = Sandbox::new();
    let model = sandbox.models().join("installed");
    package(&model, None);
    let selected = list_imported_models(&sandbox.models()).unwrap().remove(0);
    assert!(delete_imported_model(&sandbox.models(), &selected.id, Some("loaded-model")).is_err());
    assert!(model.join("model.moc3").is_file());
    delete_imported_model(&sandbox.models(), &selected.id, None).unwrap();
    assert!(!model.exists());
}

#[test]
fn receipt_only_cleanup_is_listed_and_retryable() {
    let sandbox = Sandbox::new();
    let model = sandbox.models().join("installed");
    package(&model, Some("vts-model-1"));
    let selected = list_imported_models(&sandbox.models()).unwrap().remove(0);
    let receipt = sandbox
        .models()
        .join(format!(".meowlive-delete-{}.json", selected.id));
    fs::write(
        &receipt,
        json!({"name":"installed", "model_file":"model.model3.json",
        "model_id":"vts-model-1"})
        .to_string(),
    )
    .unwrap();
    fs::remove_dir_all(&model).unwrap();
    assert_eq!(list_imported_models(&sandbox.models()).unwrap().len(), 1);
    delete_imported_model(&sandbox.models(), &selected.id, None).unwrap();
    assert!(!receipt.exists());
    assert!(list_imported_models(&sandbox.models()).unwrap().is_empty());
}
