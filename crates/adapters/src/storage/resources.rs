//! 版本化资源目录快照与不可变参考 WAV 文件存储。

use meowlive_application::ports::storage::{ResourceStore, ResourceStoreError, StoredReference};
use meowlive_domain::resources::{ReferenceAsset, ResourceCatalog};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

mod snapshot;
mod wav;

use snapshot::CatalogSnapshot;
use wav::validate_reference_wav;

pub(super) const MAX_REFERENCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_CATALOG_BYTES: usize = 512 * 1024;

#[derive(Clone, Debug)]
pub struct FileResourceStoreConfig {
    pub storage_root: PathBuf,
    pub engine_root: String,
}

pub struct FileResourceStore {
    storage_root: PathBuf,
    references_root: PathBuf,
    engine_root: PathBuf,
}

impl FileResourceStore {
    pub fn new(config: FileResourceStoreConfig) -> Result<Self, ResourceStoreError> {
        if !config.storage_root.is_absolute() || !safe_absolute(Path::new(&config.engine_root)) {
            return Err(store_error("资源根和引擎根必须是规范化绝对路径"));
        }
        fs::create_dir_all(&config.storage_root)
            .map_err(|_| store_error("无法创建资源存储根目录"))?;
        let storage_root = config
            .storage_root
            .canonicalize()
            .map_err(|_| store_error("无法解析资源存储根目录"))?;
        let references_root = storage_root.join("references");
        fs::create_dir_all(&references_root).map_err(|_| store_error("无法创建参考音频目录"))?;
        let store = Self {
            storage_root,
            references_root,
            engine_root: PathBuf::from(config.engine_root),
        };
        store.ensure_directories()?;
        Ok(store)
    }

    fn catalog_path(&self) -> PathBuf {
        self.storage_root.join("catalog.json")
    }

    fn reference_path(&self, reference: &ReferenceAsset) -> Result<PathBuf, ResourceStoreError> {
        validate_uuid(reference.as_str())?;
        Ok(self
            .references_root
            .join(format!("{}.wav", reference.as_str())))
    }

    fn ensure_directories(&self) -> Result<(), ResourceStoreError> {
        ensure_directory(&self.storage_root, None)?;
        ensure_directory(&self.references_root, Some(&self.storage_root))
    }
}

impl ResourceStore for FileResourceStore {
    fn load(&self) -> Result<ResourceCatalog, ResourceStoreError> {
        self.ensure_directories()?;
        let path = self.catalog_path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ResourceCatalog::default());
            }
            Err(_) => return Err(store_error("无法读取资源目录快照")),
        };
        if !metadata.file_type().is_file() || metadata.len() > MAX_CATALOG_BYTES as u64 {
            return Err(store_error("资源目录快照无效或超过大小上限"));
        }
        let file = File::open(path).map_err(|_| store_error("无法打开资源目录快照"))?;
        let mut bytes = Vec::new();
        file.take(MAX_CATALOG_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| store_error("无法读取资源目录快照"))?;
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(store_error("资源目录快照超过大小上限"));
        }
        let snapshot: CatalogSnapshot =
            serde_json::from_slice(&bytes).map_err(|_| store_error("资源目录快照格式损坏"))?;
        snapshot.into_domain()
    }

    fn save(&self, catalog: &ResourceCatalog) -> Result<(), ResourceStoreError> {
        self.ensure_directories()?;
        if catalog
            .voices
            .iter()
            .any(|voice| voice.reference.as_str() != voice.id)
        {
            return Err(store_error("参考音频标识必须与音色标识一致"));
        }
        catalog
            .validate()
            .map_err(|_| store_error("资源目录内容无效"))?;
        let bytes = serde_json::to_vec_pretty(&CatalogSnapshot::from_domain(catalog))
            .map_err(|_| store_error("无法编码资源目录快照"))?;
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(store_error("资源目录快照超过大小上限"));
        }
        let temporary = self
            .storage_root
            .join(format!(".catalog-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|_| store_error("无法创建资源目录临时快照"))?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|_| store_error("无法写入资源目录临时快照"))?;
            fs::rename(&temporary, self.catalog_path())
                .map_err(|_| store_error("无法发布资源目录快照"))?;
            // Rename is the commit point; after it the new snapshot is observable.
            let _ = File::open(&self.storage_root).and_then(|directory| directory.sync_all());
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }

    fn new_voice_id(&self) -> Result<String, ResourceStoreError> {
        Ok(uuid::Uuid::new_v4().to_string())
    }

    fn store_reference(
        &self,
        voice_id: &str,
        wav: &[u8],
    ) -> Result<StoredReference, ResourceStoreError> {
        self.ensure_directories()?;
        validate_uuid(voice_id)?;
        let metadata = validate_reference_wav(wav)?;
        let reference =
            ReferenceAsset::new(voice_id).map_err(|_| store_error("无法创建参考音频标识"))?;
        let path = self.reference_path(&reference)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| store_error("参考音频标识冲突或文件无法创建"))?;
        if file.write_all(wav).and_then(|()| file.sync_all()).is_err() {
            drop(file);
            let _ = fs::remove_file(path);
            return Err(store_error("无法写入参考音频文件"));
        }
        Ok(StoredReference {
            reference,
            audio: metadata,
        })
    }

    fn remove_reference(&self, reference: &ReferenceAsset) -> Result<(), ResourceStoreError> {
        self.ensure_directories()?;
        let path = self.reference_path(reference)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(store_error("无法清理未发布的参考音频")),
        }
    }

    fn resolve_reference(&self, reference: &ReferenceAsset) -> Result<String, ResourceStoreError> {
        self.ensure_directories()?;
        let local = self.reference_path(reference)?;
        let metadata = fs::symlink_metadata(&local).map_err(|_| {
            store_error(format!(
                "参考音频 references/{}.wav 不存在",
                reference.as_str()
            ))
        })?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_REFERENCE_BYTES as u64 {
            return Err(store_error(format!(
                "参考音频 references/{}.wav 不可用",
                reference.as_str()
            )));
        }
        let engine_path = self
            .engine_root
            .join("references")
            .join(format!("{}.wav", reference.as_str()));
        engine_path
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| store_error("引擎参考音频路径无效"))
    }
}

fn safe_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::Normal(_)
            )
        })
}

fn ensure_directory(path: &Path, parent: Option<&Path>) -> Result<(), ResourceStoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| store_error("资源目录不可用"))?;
    if !metadata.file_type().is_dir() {
        return Err(store_error("资源目录不能是符号链接或普通文件"));
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| store_error("无法解析资源目录"))?;
    if canonical != path {
        return Err(store_error("资源目录路径已被替换"));
    }
    if parent.is_some_and(|parent| canonical.parent() != Some(parent)) {
        return Err(store_error("参考音频目录已离开资源根目录"));
    }
    Ok(())
}

pub(super) fn validate_uuid(value: &str) -> Result<(), ResourceStoreError> {
    let parsed = uuid::Uuid::parse_str(value).map_err(|_| store_error("参考音频标识无效"))?;
    if parsed.hyphenated().to_string() != value {
        return Err(store_error("参考音频标识无效"));
    }
    Ok(())
}

pub(super) fn store_error(message: impl Into<String>) -> ResourceStoreError {
    ResourceStoreError::new(message)
}
