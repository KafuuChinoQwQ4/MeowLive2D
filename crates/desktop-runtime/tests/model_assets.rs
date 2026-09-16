use meowlive_desktop_runtime::assets::{
    AssetError, ModelImportLimits, install_model_package, validate_model_package,
};
use serde_json::json;
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
            .join("../../target/model-assets-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn complete_package(root: &Path) {
    write(root.join("model.moc3"), b"moc3");
    write(root.join("textures/texture_00.png"), b"png");
    write(root.join("model.physics3.json"), b"{}");
    write(root.join("expressions/happy.exp3.json"), b"{}");
    write(root.join("expressions/preview-only.exp3.json"), b"{}");
    write(root.join("motions/wave.motion3.json"), b"{}");
    write(
        root.join("model.model3.json"),
        json!({
            "Version": 3,
            "FileReferences": {
                "Moc": "model.moc3",
                "Textures": ["textures/texture_00.png"],
                "Physics": "model.physics3.json",
                "Expressions": [{"Name":"happy", "File":"expressions/happy.exp3.json"}],
                "Motions": {"Wave":[{"File":"motions/wave.motion3.json"}]}
            }
        })
        .to_string(),
    );
    write(
        root.join("model.vtube.json"),
        json!({
            "FileReferences": {
                "Model":"model.model3.json",
                "Icon":"",
                "IdleAnimation":"motions/wave.motion3.json",
                "IdleAnimationWhenTrackingLost":"motions/wave.motion3.json"
            },
            "Hotkeys":[{"Action":"ToggleExpression","File":"preview-only.exp3.json",
                "Folder":"expressions"}]
        })
        .to_string(),
    );
}

#[test]
fn validates_wrapped_package_and_installs_atomically_without_overwrite() {
    let sandbox = Sandbox::new();
    let selected = sandbox.path().join("selected");
    let package_root = selected.join("character");
    complete_package(&package_root);
    let models = sandbox.path().join("StreamingAssets/Live2DModels");
    fs::create_dir_all(&models).unwrap();

    let package = validate_model_package(&selected, &ModelImportLimits::default()).unwrap();
    assert_eq!(package.root(), package_root);
    assert_eq!(package.model_file(), Path::new("model.model3.json"));
    assert_eq!(package.referenced_files().len(), 7);
    assert_eq!(package.file_count(), 8);
    assert!(package.total_bytes() > 0);

    let installed = install_model_package(&package, &models, "魔女").unwrap();
    assert_eq!(installed.installed_path, models.join("魔女"));
    assert_eq!(installed.model_file, Path::new("model.model3.json"));
    assert_eq!(installed.file_count, package.file_count());
    assert_eq!(installed.total_bytes, package.total_bytes());
    assert!(installed.restart_required);
    assert_eq!(fs::read(models.join("魔女/model.moc3")).unwrap(), b"moc3");
    assert!(package_root.join("model.moc3").exists());

    let error = install_model_package(&package, &models, "魔女").unwrap_err();
    assert_eq!(error, AssetError::AlreadyExists);
    assert_eq!(
        fs::read_dir(&models)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".meowlive-import-"))
            .count(),
        0
    );
}

#[test]
fn missing_resource_error_identifies_the_relative_file() {
    let sandbox = Sandbox::new();
    complete_package(sandbox.path());
    fs::remove_file(sandbox.path().join("expressions/happy.exp3.json")).unwrap();
    let error = validate_model_package(sandbox.path(), &ModelImportLimits::default()).unwrap_err();
    assert!(
        error.to_string().contains("expressions/happy.exp3.json"),
        "{error}"
    );
}

#[test]
fn rejects_unsafe_or_incomplete_packages_and_resource_limits() {
    let sandbox = Sandbox::new();
    let package = sandbox.path().join("package");
    complete_package(&package);

    fs::remove_file(package.join("model.physics3.json")).unwrap();
    assert!(matches!(
        validate_model_package(&package, &ModelImportLimits::default()),
        Err(AssetError::MissingReference(_))
    ));
    complete_package(&package);
    write(
        package.join("model.model3.json"),
        json!({"Version":3,"FileReferences":{
            "Moc":"../outside.moc3","Textures":["textures/texture_00.png"]
        }})
        .to_string(),
    );
    assert_eq!(
        validate_model_package(&package, &ModelImportLimits::default()).unwrap_err(),
        AssetError::UnsafeReference("../outside.moc3".into())
    );

    complete_package(&package);
    let limits = ModelImportLimits {
        max_files: 5,
        ..ModelImportLimits::default()
    };
    assert_eq!(
        validate_model_package(&package, &limits).unwrap_err(),
        AssetError::FileCountLimit
    );

    let limits = ModelImportLimits {
        max_file_bytes: 3,
        ..ModelImportLimits::default()
    };
    assert_eq!(
        validate_model_package(&package, &limits).unwrap_err(),
        AssetError::FileSizeLimit
    );
    let limits = ModelImportLimits {
        max_total_bytes: 20,
        ..ModelImportLimits::default()
    };
    assert_eq!(
        validate_model_package(&package, &limits).unwrap_err(),
        AssetError::TotalSizeLimit
    );
    write(package.join("nested/deeper/unreferenced.bin"), b"x");
    let limits = ModelImportLimits {
        max_depth: 1,
        ..ModelImportLimits::default()
    };
    assert_eq!(
        validate_model_package(&package, &limits).unwrap_err(),
        AssetError::DepthLimit
    );
}

#[test]
fn rejects_invalid_vtube_model_and_missing_hotkey_references() {
    let sandbox = Sandbox::new();
    let package = sandbox.path().join("package");
    complete_package(&package);
    write(
        package.join("model.vtube.json"),
        json!({"FileReferences":{"Model":"model.physics3.json"},"Hotkeys":[]}).to_string(),
    );
    assert_eq!(
        validate_model_package(&package, &ModelImportLimits::default()).unwrap_err(),
        AssetError::InvalidModelData
    );

    complete_package(&package);
    fs::remove_file(package.join("expressions/preview-only.exp3.json")).unwrap();
    let error = validate_model_package(&package, &ModelImportLimits::default()).unwrap_err();
    assert_eq!(
        error,
        AssetError::MissingReference("expressions/preview-only.exp3.json".into())
    );
    assert!(
        error
            .to_string()
            .contains("expressions/preview-only.exp3.json")
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlinks_in_source_and_destination() {
    use std::os::unix::fs::symlink;

    let sandbox = Sandbox::new();
    let package = sandbox.path().join("package");
    complete_package(&package);
    symlink(package.join("model.moc3"), package.join("alias.moc3")).unwrap();
    assert_eq!(
        validate_model_package(&package, &ModelImportLimits::default()).unwrap_err(),
        AssetError::SymlinkNotAllowed
    );

    fs::remove_file(package.join("alias.moc3")).unwrap();
    let validated = validate_model_package(&package, &ModelImportLimits::default()).unwrap();
    let real_models = sandbox.path().join("real/Live2DModels");
    fs::create_dir_all(&real_models).unwrap();
    let linked_models = sandbox.path().join("Live2DModels");
    symlink(&real_models, &linked_models).unwrap();
    assert_eq!(
        install_model_package(&validated, &linked_models, "model").unwrap_err(),
        AssetError::SymlinkNotAllowed
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlinks_in_source_or_destination_ancestors() {
    use std::os::unix::fs::symlink;

    let sandbox = Sandbox::new();
    let real_parent = sandbox.path().join("real-source");
    let real_package = real_parent.join("package");
    complete_package(&real_package);
    let linked_parent = sandbox.path().join("linked-source");
    symlink(&real_parent, &linked_parent).unwrap();
    assert_eq!(
        validate_model_package(
            &linked_parent.join("package"),
            &ModelImportLimits::default()
        )
        .unwrap_err(),
        AssetError::SymlinkNotAllowed
    );

    let validated = validate_model_package(&real_package, &ModelImportLimits::default()).unwrap();
    let real_target_parent = sandbox.path().join("real-target");
    let real_models = real_target_parent.join("Live2DModels");
    fs::create_dir_all(&real_models).unwrap();
    let linked_target_parent = sandbox.path().join("linked-target");
    symlink(&real_target_parent, &linked_target_parent).unwrap();
    assert_eq!(
        install_model_package(
            &validated,
            &linked_target_parent.join("Live2DModels"),
            "model"
        )
        .unwrap_err(),
        AssetError::SymlinkNotAllowed
    );
}

#[test]
fn only_installs_into_live2dmodels_with_a_safe_new_folder_name() {
    let sandbox = Sandbox::new();
    let package_root = sandbox.path().join("package");
    complete_package(&package_root);
    let package = validate_model_package(&package_root, &ModelImportLimits::default()).unwrap();
    let wrong_root = sandbox.path().join("Models");
    fs::create_dir_all(&wrong_root).unwrap();

    assert_eq!(
        install_model_package(&package, &wrong_root, "model").unwrap_err(),
        AssetError::InvalidModelsDirectory
    );
    let models = sandbox.path().join("Live2DModels");
    fs::create_dir_all(&models).unwrap();
    assert_eq!(
        install_model_package(&package, &models, "../escape").unwrap_err(),
        AssetError::InvalidInstallName
    );
}

#[test]
#[ignore = "requires an explicitly authorized local model path"]
fn validates_and_installs_an_authorized_external_model_package() {
    let source = std::env::var_os("MEOWLIVE_MODEL_PACKAGE")
        .expect("set MEOWLIVE_MODEL_PACKAGE to the authorized local model directory");
    let models =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/m4-resources/Live2DModels");
    fs::create_dir_all(&models).unwrap();
    let name = format!("authorized-smoke-{}", std::process::id());
    let installed_path = models.join(&name);
    let _ = fs::remove_dir_all(&installed_path);

    let package =
        validate_model_package(Path::new(&source), &ModelImportLimits::default()).unwrap();
    let installed = install_model_package(&package, &models, &name).unwrap();
    assert!(
        installed
            .installed_path
            .join(&installed.model_file)
            .is_file()
    );
    assert!(installed.file_count > 0);
    assert!(installed.total_bytes > 0);

    fs::remove_dir_all(installed_path).unwrap();
}
