use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const DEFAULT_MAX_FILES: u32 = 4_096;
const DEFAULT_MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const DEFAULT_MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const DEFAULT_MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const DEFAULT_MAX_DEPTH: usize = 16;
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct ModelImportLimits {
    pub max_files: u32,
    pub max_total_bytes: u64,
    pub max_file_bytes: u64,
    pub max_manifest_bytes: u64,
    pub max_depth: usize,
}

impl Default for ModelImportLimits {
    fn default() -> Self {
        Self {
            max_files: DEFAULT_MAX_FILES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_manifest_bytes: DEFAULT_MAX_MANIFEST_BYTES,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValidatedModelPackage {
    pub package_root: PathBuf,
    model_file: PathBuf,
    referenced_files: Vec<PathBuf>,
    files: Vec<ScannedFile>,
    pub file_count: u32,
    pub total_bytes: u32,
    limits: ModelImportLimits,
}

impl ValidatedModelPackage {
    pub fn root(&self) -> &Path {
        &self.package_root
    }

    pub fn model_file(&self) -> &Path {
        &self.model_file
    }

    pub fn referenced_files(&self) -> &[PathBuf] {
        &self.referenced_files
    }

    pub fn file_count(&self) -> u32 {
        self.file_count
    }

    pub fn total_bytes(&self) -> u32 {
        self.total_bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledModel {
    pub installed_path: PathBuf,
    pub model_file: PathBuf,
    pub file_count: u32,
    pub total_bytes: u32,
    pub restart_required: bool,
}

/// A model directory directly installed below VTube Studio's Live2DModels.
/// The identifier is opaque and is never interpreted as a filesystem path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedModel {
    pub id: String,
    pub name: String,
    pub model_file: PathBuf,
    pub model_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetError {
    InvalidSource,
    ModelManifestMissing,
    MultipleModelManifests,
    InvalidManifest,
    InvalidModelData,
    UnsafeReference(String),
    MissingReference(String),
    SymlinkNotAllowed,
    UnsupportedEntry,
    FileCountLimit,
    FileSizeLimit,
    TotalSizeLimit,
    DepthLimit,
    InvalidModelsDirectory,
    InvalidInstallName,
    InvalidModelId,
    ModelInUse,
    DeleteIncomplete,
    AlreadyExists,
    SourceChanged,
    Io,
}

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafeReference(path) => {
                return write!(formatter, "模型清单包含不安全的文件引用：{path:?}");
            }
            Self::MissingReference(path) => {
                return write!(formatter, "模型清单引用的文件不存在：{path:?}");
            }
            _ => {}
        }
        let message = match self {
            Self::InvalidSource => "模型包目录无效",
            Self::ModelManifestMissing => "模型包中缺少 .model3.json",
            Self::MultipleModelManifests => "模型包中只能包含一个 .model3.json",
            Self::InvalidManifest => "模型清单格式无效",
            Self::InvalidModelData => "模型清单缺少有效的 moc3 或纹理引用",
            Self::UnsafeReference(_) | Self::MissingReference(_) => unreachable!(),
            Self::SymlinkNotAllowed => "模型路径不能包含符号链接",
            Self::UnsupportedEntry => "模型包包含不支持的文件类型",
            Self::FileCountLimit => "模型包文件数量超过限制",
            Self::FileSizeLimit => "模型包单个文件超过大小限制",
            Self::TotalSizeLimit => "模型包总大小超过限制",
            Self::DepthLimit => "模型包目录层级超过限制",
            Self::InvalidModelsDirectory => "安装目标必须是现存的 Live2DModels 目录",
            Self::InvalidInstallName => "模型安装目录名称无效",
            Self::InvalidModelId => "模型安装标识无效",
            Self::ModelInUse => {
                "目标模型正在 VTS 中使用，或暂时无法确认模型身份；请先卸载当前 VTS 模型后再删除"
            }
            Self::DeleteIncomplete => "模型目录清理未完成，请释放文件占用后重试删除",
            Self::AlreadyExists => "同名 VTS 模型目录已存在",
            Self::SourceChanged => "模型包在校验后发生变化",
            Self::Io => "模型资源文件操作失败",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AssetError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScannedFile {
    relative: PathBuf,
    bytes: u64,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "FileReferences")]
    files: FileReferences,
}

#[derive(Deserialize)]
struct FileReferences {
    #[serde(rename = "Moc")]
    moc: String,
    #[serde(rename = "Textures")]
    textures: Vec<String>,
    #[serde(rename = "Physics", default)]
    physics: Option<String>,
    #[serde(rename = "Pose", default)]
    pose: Option<String>,
    #[serde(rename = "DisplayInfo", default)]
    display_info: Option<String>,
    #[serde(rename = "UserData", default)]
    user_data: Option<String>,
    #[serde(rename = "Expressions", default)]
    expressions: Vec<ExpressionReference>,
    #[serde(rename = "Motions", default)]
    motions: BTreeMap<String, Vec<MotionReference>>,
}

#[derive(Deserialize)]
struct ExpressionReference {
    #[serde(rename = "File")]
    file: String,
}

#[derive(Deserialize)]
struct MotionReference {
    #[serde(rename = "File")]
    file: String,
    #[serde(rename = "Sound", default)]
    sound: Option<String>,
}

#[derive(Deserialize)]
struct VtubeConfig {
    #[serde(rename = "ModelID", default)]
    model_id: Option<String>,
    #[serde(rename = "FileReferences")]
    files: VtubeFileReferences,
    #[serde(rename = "Hotkeys", default)]
    hotkeys: Vec<VtubeHotkey>,
}

#[derive(Deserialize)]
struct VtubeFileReferences {
    #[serde(rename = "Model")]
    model: String,
    #[serde(rename = "Icon", default)]
    icon: Option<String>,
    #[serde(rename = "IdleAnimation", default)]
    idle_animation: Option<String>,
    #[serde(rename = "IdleAnimationWhenTrackingLost", default)]
    idle_animation_when_tracking_lost: Option<String>,
}

#[derive(Deserialize)]
struct VtubeHotkey {
    #[serde(rename = "Action", default)]
    action: String,
    #[serde(rename = "File", default)]
    file: String,
    #[serde(rename = "Folder", default)]
    folder: String,
}

pub fn validate_model_package(
    selected: &Path,
    limits: &ModelImportLimits,
) -> Result<ValidatedModelPackage, AssetError> {
    validate_limits(*limits)?;
    reject_symlink_ancestors(selected)?;
    let manifests = find_manifests(selected, limits.max_depth)?;
    let manifest_path = match manifests.as_slice() {
        [] => return Err(AssetError::ModelManifestMissing),
        [manifest] => manifest,
        _ => return Err(AssetError::MultipleModelManifests),
    };
    let root = manifest_path
        .parent()
        .ok_or(AssetError::InvalidSource)?
        .to_path_buf();
    let model_file = manifest_path
        .strip_prefix(&root)
        .map_err(|_| AssetError::InvalidSource)?
        .to_path_buf();
    let (files, total_bytes) = scan_package(&root, *limits)?;
    let manifest_size = files
        .iter()
        .find(|file| file.relative == model_file)
        .ok_or(AssetError::ModelManifestMissing)?
        .bytes;
    if manifest_size > limits.max_manifest_bytes {
        return Err(AssetError::FileSizeLimit);
    }
    let manifest_bytes = read_json_file(manifest_path, limits.max_manifest_bytes)?;
    let manifest: Manifest =
        serde_json::from_slice(&manifest_bytes).map_err(|_| AssetError::InvalidManifest)?;
    let mut referenced_files = validate_references(&root, manifest.files, limits.max_depth)?;
    referenced_files.extend(validate_vtube_references(
        &root,
        &model_file,
        limits.max_manifest_bytes,
        limits.max_depth,
    )?);
    referenced_files.sort();
    referenced_files.dedup();
    let file_count = files.len() as u32;
    Ok(ValidatedModelPackage {
        package_root: root,
        model_file,
        referenced_files,
        files,
        file_count,
        total_bytes,
        limits: *limits,
    })
}

pub fn install_model_package(
    package: &ValidatedModelPackage,
    live2d_models_dir: &Path,
    install_name: &str,
) -> Result<InstalledModel, AssetError> {
    validate_models_directory(live2d_models_dir)?;
    validate_install_name(install_name)?;
    let refreshed = validate_model_package(&package.package_root, &package.limits)?;
    if refreshed.model_file != package.model_file
        || refreshed.files != package.files
        || refreshed.total_bytes != package.total_bytes
    {
        return Err(AssetError::SourceChanged);
    }
    let source = fs::canonicalize(&package.package_root).map_err(|_| AssetError::Io)?;
    let destination_root =
        fs::canonicalize(live2d_models_dir).map_err(|_| AssetError::InvalidModelsDirectory)?;
    if destination_root.starts_with(&source) {
        return Err(AssetError::InvalidModelsDirectory);
    }

    let _lock = ImportLock::acquire(live2d_models_dir)?;
    let destination = live2d_models_dir.join(install_name);
    if fs::symlink_metadata(delete_receipt_path(
        live2d_models_dir,
        &imported_model_id(install_name),
    ))
    .is_ok()
    {
        return Err(AssetError::DeleteIncomplete);
    }
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(AssetError::AlreadyExists);
    }
    let stage = create_stage(live2d_models_dir)?;
    for file in &refreshed.files {
        let source_file = refreshed.package_root.join(&file.relative);
        let metadata = fs::symlink_metadata(&source_file).map_err(|_| AssetError::SourceChanged)?;
        if !metadata.is_file() || metadata.len() != file.bytes {
            return Err(AssetError::SourceChanged);
        }
        let target_file = stage.path.join(&file.relative);
        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent).map_err(|_| AssetError::Io)?;
        }
        let copied = fs::copy(&source_file, &target_file).map_err(|_| AssetError::Io)?;
        if copied != file.bytes {
            return Err(AssetError::SourceChanged);
        }
        fs::File::open(&target_file)
            .and_then(|file| file.sync_all())
            .map_err(|_| AssetError::Io)?;
    }
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(AssetError::AlreadyExists);
    }
    fs::rename(&stage.path, &destination).map_err(|error| {
        if fs::symlink_metadata(&destination).is_ok() {
            AssetError::AlreadyExists
        } else {
            let _ = error;
            AssetError::Io
        }
    })?;
    stage.commit();
    let file_count = refreshed.file_count;
    Ok(InstalledModel {
        installed_path: destination,
        model_file: refreshed.model_file,
        file_count,
        total_bytes: refreshed.total_bytes,
        restart_required: true,
    })
}

/// Enumerate only direct child directories of the configured Live2DModels directory.
/// This intentionally includes legacy installs without a receipt, while exposing no path.
pub fn list_imported_models(live2d_models_dir: &Path) -> Result<Vec<ImportedModel>, AssetError> {
    validate_models_directory(live2d_models_dir)?;
    let mut models = Vec::new();
    for entry in fs::read_dir(live2d_models_dir).map_err(|_| AssetError::Io)? {
        let entry = entry.map_err(|_| AssetError::Io)?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if let Some(id) = name
            .strip_prefix(".meowlive-delete-")
            .and_then(|s| s.strip_suffix(".json"))
        {
            if let Ok(model) = read_receipt_by_id(live2d_models_dir, id)
                && fs::symlink_metadata(live2d_models_dir.join(&model.name))
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            {
                models.push(model);
            }
            continue;
        }
        if name.starts_with(".meowlive-") || validate_install_name(&name).is_err() {
            continue;
        }
        // VTS also keeps unrelated folders here. An invalid package must not hide
        // the valid models, and it must never become a remote deletion candidate.
        if let Ok(model) = inspect_imported_model(&entry.path(), &name) {
            models.push(model);
        }
    }
    models.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(models)
}

/// Remove an installed model selected by its opaque ID. The source package is untouched.
pub fn delete_imported_model(
    live2d_models_dir: &Path,
    id: &str,
    current_model_id: Option<&str>,
) -> Result<ImportedModel, AssetError> {
    validate_models_directory(live2d_models_dir)?;
    if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AssetError::InvalidModelId);
    }
    let _lock = ImportLock::acquire(live2d_models_dir)?;
    let destination_root =
        fs::canonicalize(live2d_models_dir).map_err(|_| AssetError::InvalidModelsDirectory)?;
    let mut selected = None;
    for entry in fs::read_dir(live2d_models_dir).map_err(|_| AssetError::Io)? {
        let entry = entry.map_err(|_| AssetError::Io)?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.starts_with(".meowlive-")
            && validate_install_name(&name).is_ok()
            && imported_model_id(&name) == id
        {
            selected = Some((entry.path(), name));
            break;
        }
    }
    let Some((path, name)) = selected else {
        let pending = read_receipt_by_id(live2d_models_dir, id)?;
        if !fs::symlink_metadata(live2d_models_dir.join(&pending.name))
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            return Err(AssetError::InvalidModelId);
        }
        fs::remove_file(delete_receipt_path(live2d_models_dir, id))
            .map_err(|_| AssetError::DeleteIncomplete)?;
        return Ok(pending);
    };
    // Inspect before removing anything: all descendants and ancestors reject
    // symlinks/reparse points, and the manifest/resources must still be valid.
    let model = inspect_imported_model(&path, &name)?;
    if current_model_id
        .is_some_and(|loaded| model.model_id.as_deref().is_none_or(|id| id == loaded))
    {
        return Err(AssetError::ModelInUse);
    }
    let canonical = fs::canonicalize(&path).map_err(|_| AssetError::Io)?;
    if canonical.parent() != Some(destination_root.as_path()) {
        return Err(AssetError::InvalidModelsDirectory);
    }
    let receipt_path = delete_receipt_path(live2d_models_dir, id);
    if !receipt_path.try_exists().map_err(|_| AssetError::Io)? {
        let receipt = DeleteReceipt {
            name: model.name.clone(),
            model_file: model.model_file.to_string_lossy().into_owned(),
            model_id: model.model_id.clone(),
        };
        let stage = create_stage(live2d_models_dir)?;
        let staged_receipt = stage.path.join("delete.json");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_receipt)
            .map_err(|_| AssetError::Io)?;
        serde_json::to_writer(&mut file, &receipt).map_err(|_| AssetError::Io)?;
        file.sync_all().map_err(|_| AssetError::Io)?;
        drop(file);
        fs::rename(&staged_receipt, &receipt_path).map_err(|_| AssetError::Io)?;
    }
    // The receipt lives outside the package so partial filesystem failures or a
    // process interruption cannot remove the retry entry along with the manifest.
    fs::remove_dir_all(&path).map_err(|_| AssetError::DeleteIncomplete)?;
    fs::remove_file(receipt_path).map_err(|_| AssetError::DeleteIncomplete)?;
    Ok(model)
}

fn imported_model_id(name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"meowlive2d-installed-model\0");
    hasher.update(name.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn inspect_imported_model(root: &Path, name: &str) -> Result<ImportedModel, AssetError> {
    reject_symlink_ancestors(root)?;
    let metadata = fs::symlink_metadata(root).map_err(|_| AssetError::InvalidSource)?;
    if !metadata.is_dir() {
        return Err(AssetError::InvalidSource);
    }
    let limits = ModelImportLimits::default();
    // Enforce traversal limits before the manifest discovery walk. Model binary
    // data is only stat'ed, never loaded into memory for list/delete operations.
    let (files, _) = scan_package(root, limits)?;
    if let Some(pending) = read_delete_receipt(root, name)? {
        return Ok(pending);
    }
    let package = validate_model_package(root, &limits)?;
    if package.package_root != root {
        return Err(AssetError::InvalidSource);
    }
    let mut ids = BTreeSet::new();
    for file in files.iter().filter(|file| {
        file.relative.components().count() == 1
            && file
                .relative
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".vtube.json")
    }) {
        if file.bytes > limits.max_manifest_bytes {
            return Err(AssetError::FileSizeLimit);
        }
        let config: VtubeConfig = serde_json::from_slice(&read_json_file(
            &root.join(&file.relative),
            limits.max_manifest_bytes,
        )?)
        .map_err(|_| AssetError::InvalidManifest)?;
        if let Some(id) = config.model_id.filter(|id| !id.is_empty()) {
            if id.len() > 1024 || id.chars().any(char::is_control) {
                return Err(AssetError::InvalidManifest);
            }
            ids.insert(id);
        }
    }
    if ids.len() > 1 {
        return Err(AssetError::InvalidManifest);
    }
    Ok(ImportedModel {
        id: imported_model_id(name),
        name: name.to_owned(),
        model_file: package.model_file,
        model_id: ids.into_iter().next(),
    })
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeleteReceipt {
    name: String,
    model_file: String,
    model_id: Option<String>,
}

fn delete_receipt_path(root: &Path, id: &str) -> PathBuf {
    root.join(format!(".meowlive-delete-{id}.json"))
}

fn read_receipt_by_id(root: &Path, id: &str) -> Result<ImportedModel, AssetError> {
    if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AssetError::InvalidModelId);
    }
    let path = delete_receipt_path(root, id);
    let metadata = fs::symlink_metadata(&path).map_err(|_| AssetError::InvalidModelId)?;
    if metadata_is_link(&metadata) {
        return Err(AssetError::SymlinkNotAllowed);
    }
    if !metadata.is_file() {
        return Err(AssetError::InvalidManifest);
    }
    let receipt: DeleteReceipt = serde_json::from_slice(&read_json_file(&path, 4096)?)
        .map_err(|_| AssetError::InvalidManifest)?;
    validate_install_name(&receipt.name)?;
    if receipt.name.starts_with(".meowlive-") || imported_model_id(&receipt.name) != id {
        return Err(AssetError::InvalidModelId);
    }
    read_delete_receipt(&root.join(&receipt.name), &receipt.name)?.ok_or(AssetError::InvalidModelId)
}

fn read_delete_receipt(root: &Path, name: &str) -> Result<Option<ImportedModel>, AssetError> {
    let id = imported_model_id(name);
    let path = delete_receipt_path(root.parent().ok_or(AssetError::InvalidSource)?, &id);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(AssetError::Io),
    };
    if metadata_is_link(&metadata) {
        return Err(AssetError::SymlinkNotAllowed);
    }
    if !metadata.is_file() {
        return Err(AssetError::InvalidManifest);
    }
    let receipt: DeleteReceipt = serde_json::from_slice(&read_json_file(&path, 4096)?)
        .map_err(|_| AssetError::InvalidManifest)?;
    if receipt.name != name
        || validate_install_name(&receipt.model_file).is_err()
        || !receipt
            .model_file
            .to_ascii_lowercase()
            .ends_with(".model3.json")
        || receipt
            .model_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 1024 || id.chars().any(char::is_control))
    {
        return Err(AssetError::InvalidManifest);
    }
    Ok(Some(ImportedModel {
        id,
        name: name.to_owned(),
        model_file: receipt.model_file.into(),
        model_id: receipt.model_id,
    }))
}

fn read_json_file(path: &Path, limit: u64) -> Result<Vec<u8>, AssetError> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| AssetError::Io)?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| AssetError::Io)?;
    if bytes.len() as u64 > limit {
        return Err(AssetError::FileSizeLimit);
    }
    Ok(bytes)
}

fn metadata_is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Also reject junctions and other reparse points on Windows.
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

fn validate_limits(limits: ModelImportLimits) -> Result<(), AssetError> {
    if limits.max_files == 0
        || limits.max_total_bytes == 0
        || limits.max_total_bytes > u32::MAX as u64
        || limits.max_file_bytes == 0
        || limits.max_manifest_bytes == 0
        || limits.max_depth == 0
    {
        return Err(AssetError::InvalidSource);
    }
    Ok(())
}

fn find_manifests(root: &Path, max_depth: usize) -> Result<Vec<PathBuf>, AssetError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| AssetError::InvalidSource)?;
    if metadata_is_link(&metadata) {
        return Err(AssetError::SymlinkNotAllowed);
    }
    if !metadata.is_dir() {
        return Err(AssetError::InvalidSource);
    }
    let mut directories = vec![(root.to_path_buf(), 0_usize)];
    let mut manifests = Vec::new();
    while let Some((directory, depth)) = directories.pop() {
        if depth > max_depth {
            return Err(AssetError::DepthLimit);
        }
        for entry in fs::read_dir(directory).map_err(|_| AssetError::Io)? {
            let entry = entry.map_err(|_| AssetError::Io)?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(|_| AssetError::Io)?;
            let kind = metadata.file_type();
            if metadata_is_link(&metadata) {
                return Err(AssetError::SymlinkNotAllowed);
            }
            if kind.is_dir() {
                directories.push((entry.path(), depth + 1));
            } else if kind.is_file() {
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                if name.ends_with(".model3.json") {
                    manifests.push(entry.path());
                }
            } else {
                return Err(AssetError::UnsupportedEntry);
            }
        }
    }
    Ok(manifests)
}

fn scan_package(
    root: &Path,
    limits: ModelImportLimits,
) -> Result<(Vec<ScannedFile>, u32), AssetError> {
    let mut directories = vec![(root.to_path_buf(), 0_usize)];
    let mut files = Vec::new();
    let mut total = 0_u64;
    while let Some((directory, depth)) = directories.pop() {
        if depth > limits.max_depth {
            return Err(AssetError::DepthLimit);
        }
        for entry in fs::read_dir(directory).map_err(|_| AssetError::Io)? {
            let entry = entry.map_err(|_| AssetError::Io)?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(|_| AssetError::Io)?;
            if metadata_is_link(&metadata) {
                return Err(AssetError::SymlinkNotAllowed);
            }
            if metadata.is_dir() {
                directories.push((entry.path(), depth + 1));
                continue;
            }
            if !metadata.is_file() {
                return Err(AssetError::UnsupportedEntry);
            }
            if metadata.len() > limits.max_file_bytes {
                return Err(AssetError::FileSizeLimit);
            }
            if files.len() >= limits.max_files as usize {
                return Err(AssetError::FileCountLimit);
            }
            total = total
                .checked_add(metadata.len())
                .ok_or(AssetError::TotalSizeLimit)?;
            if total > limits.max_total_bytes || total > u32::MAX as u64 {
                return Err(AssetError::TotalSizeLimit);
            }
            files.push(ScannedFile {
                relative: entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| AssetError::Io)?
                    .to_path_buf(),
                bytes: metadata.len(),
            });
        }
    }
    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok((files, total as u32))
}

fn validate_references(
    root: &Path,
    references: FileReferences,
    max_depth: usize,
) -> Result<Vec<PathBuf>, AssetError> {
    if !references.moc.to_ascii_lowercase().ends_with(".moc3") || references.textures.is_empty() {
        return Err(AssetError::InvalidModelData);
    }
    let mut values = vec![references.moc];
    values.extend(references.textures);
    values.extend(references.physics);
    values.extend(references.pose);
    values.extend(references.display_info);
    values.extend(references.user_data);
    values.extend(references.expressions.into_iter().map(|item| item.file));
    for motion in references.motions.into_values().flatten() {
        values.push(motion.file);
        values.extend(motion.sound);
    }
    let mut unique = BTreeSet::new();
    for value in values {
        let relative = safe_reference(&value, max_depth)?;
        let path = root.join(&relative);
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| AssetError::MissingReference(value.clone()))?;
        if metadata_is_link(&metadata) {
            return Err(AssetError::SymlinkNotAllowed);
        }
        if !metadata.is_file() {
            return Err(AssetError::MissingReference(value));
        }
        unique.insert(relative);
    }
    Ok(unique.into_iter().collect())
}

fn validate_vtube_references(
    root: &Path,
    model_file: &Path,
    max_json_bytes: u64,
    max_depth: usize,
) -> Result<Vec<PathBuf>, AssetError> {
    let mut validated = BTreeSet::new();
    for entry in fs::read_dir(root).map_err(|_| AssetError::Io)? {
        let entry = entry.map_err(|_| AssetError::Io)?;
        if !entry
            .file_name()
            .to_string_lossy()
            .to_ascii_lowercase()
            .ends_with(".vtube.json")
        {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| AssetError::Io)?;
        if metadata_is_link(&metadata) {
            return Err(AssetError::SymlinkNotAllowed);
        }
        if !metadata.is_file() || metadata.len() > max_json_bytes {
            return Err(AssetError::InvalidManifest);
        }
        let bytes = read_json_file(&entry.path(), max_json_bytes)?;
        let config: VtubeConfig =
            serde_json::from_slice(&bytes).map_err(|_| AssetError::InvalidManifest)?;
        let configured_model = safe_reference(&config.files.model, max_depth)?;
        if configured_model != model_file {
            return Err(AssetError::InvalidModelData);
        }
        let mut values = vec![config.files.model];
        values.extend(config.files.icon.filter(|value| !value.is_empty()));
        values.extend(
            config
                .files
                .idle_animation
                .filter(|value| !value.is_empty()),
        );
        values.extend(
            config
                .files
                .idle_animation_when_tracking_lost
                .filter(|value| !value.is_empty()),
        );
        values.extend(config.hotkeys.into_iter().filter_map(|hotkey| {
            if !matches!(
                hotkey.action.as_str(),
                "ToggleExpression" | "TriggerAnimation"
            ) || hotkey.file.is_empty()
            {
                return None;
            }
            Some(if hotkey.folder.is_empty() {
                hotkey.file
            } else {
                format!("{}/{}", hotkey.folder, hotkey.file)
            })
        }));
        for value in values {
            let relative = safe_reference(&value, max_depth)?;
            let metadata = fs::symlink_metadata(root.join(&relative))
                .map_err(|_| AssetError::MissingReference(value.clone()))?;
            if metadata_is_link(&metadata) {
                return Err(AssetError::SymlinkNotAllowed);
            }
            if !metadata.is_file() {
                return Err(AssetError::MissingReference(value));
            }
            validated.insert(relative);
        }
    }
    Ok(validated.into_iter().collect())
}

fn safe_reference(value: &str, max_depth: usize) -> Result<PathBuf, AssetError> {
    if value.is_empty()
        || value.len() > 1024
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.as_bytes().get(1) == Some(&b':')
    {
        return Err(AssetError::UnsafeReference(value.into()));
    }
    let parts: Vec<_> = value.split('/').collect();
    if parts.len() > max_depth
        || parts
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == ".." || part.contains(':'))
    {
        return Err(AssetError::UnsafeReference(value.into()));
    }
    Ok(parts.iter().collect())
}

fn validate_models_directory(path: &Path) -> Result<(), AssetError> {
    if !path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("Live2DModels"))
    {
        return Err(AssetError::InvalidModelsDirectory);
    }
    reject_symlink_ancestors(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| AssetError::InvalidModelsDirectory)?;
    if metadata_is_link(&metadata) {
        return Err(AssetError::SymlinkNotAllowed);
    }
    if !metadata.is_dir() {
        return Err(AssetError::InvalidModelsDirectory);
    }
    Ok(())
}

fn reject_symlink_ancestors(path: &Path) -> Result<(), AssetError> {
    for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata_is_link(&metadata) => {
                return Err(AssetError::SymlinkNotAllowed);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(AssetError::Io),
        }
    }
    Ok(())
}

fn validate_install_name(name: &str) -> Result<(), AssetError> {
    let mut components = Path::new(name).components();
    if name.is_empty()
        || name.len() > 128
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
    {
        return Err(AssetError::InvalidInstallName);
    }
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
    {
        return Err(AssetError::InvalidInstallName);
    }
    Ok(())
}

struct ImportLock {
    path: PathBuf,
    _file: fs::File,
}

impl ImportLock {
    fn acquire(root: &Path) -> Result<Self, AssetError> {
        let path = root.join(".meowlive-import.lock");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| AssetError::Io)?;
        write!(file, "{}", std::process::id()).map_err(|_| AssetError::Io)?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for ImportLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct Stage {
    path: PathBuf,
    committed: bool,
}

impl Stage {
    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn create_stage(root: &Path) -> Result<Stage, AssetError> {
    for _ in 0..32 {
        let path = root.join(format!(
            ".meowlive-import-{}-{}",
            std::process::id(),
            NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => {
                return Ok(Stage {
                    path,
                    committed: false,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(AssetError::Io),
        }
    }
    Err(AssetError::Io)
}
