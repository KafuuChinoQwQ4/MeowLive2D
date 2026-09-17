//! 自有目录内的不可变训练素材、原子历史快照和成对模型解析。
use meowlive_application::ports::training::{
    PreparedTrainingJob, ResolvedArtifactPair, TrainingStore,
};
use meowlive_domain::training::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
const MAX_CATALOG_BYTES: u64 = 512 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024 * 1024;
#[cfg(unix)]
type VerifiedIdentity = (u64, u64, i64, i64, i64, i64, u64, String);
pub struct FileTrainingStore {
    root: PathBuf,
    #[cfg(unix)]
    verified: std::sync::Mutex<std::collections::HashMap<PathBuf, VerifiedIdentity>>,
}
impl FileTrainingStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, TrainingError> {
        ensure_directory(root.as_ref())?;
        let root = fs::canonicalize(root.as_ref()).map_err(|_| failure())?;
        ensure_directory(&root.join("jobs"))?;
        Ok(Self {
            root,
            #[cfg(unix)]
            verified: std::sync::Mutex::new(std::collections::HashMap::new()),
        })
    }
    fn job_dir(&self, id: &str) -> Result<PathBuf, TrainingError> {
        if !valid_job_id(id) {
            return Err(TrainingError::Invalid("训练任务标识无效".into()));
        }
        checked_directory(&self.root)?;
        checked_directory(&self.root.join("jobs"))?;
        Ok(self.root.join("jobs").join(id))
    }
}
impl TrainingStore for FileTrainingStore {
    fn load(&self) -> Result<TrainingCatalog, TrainingError> {
        checked_directory(&self.root)?;
        let path = self.root.join("catalog.json");
        if !path.try_exists().map_err(|_| failure())? {
            if fs::symlink_metadata(&path).is_ok() {
                return Err(failure());
            }
            return Ok(TrainingCatalog::default());
        }
        let dto: CatalogDto = serde_json::from_slice(&read_bounded(&path, MAX_CATALOG_BYTES)?)
            .map_err(|_| failure())?;
        if dto.schema != 1 {
            return Err(failure());
        }
        let catalog = TrainingCatalog {
            jobs: dto
                .jobs
                .into_iter()
                .map(|job| {
                    let active = dto.active_versions.get(&job.voice_id) == Some(&job.id);
                    job.into_domain(active)
                })
                .collect::<Result<_, _>>()?,
            active_versions: dto.active_versions,
        };
        catalog.validate()?;
        for job in &catalog.jobs {
            checked_directory(&self.job_dir(&job.id)?)?;
        }
        Ok(catalog)
    }
    fn save(&self, catalog: &TrainingCatalog) -> Result<(), TrainingError> {
        catalog.validate()?;
        checked_directory(&self.root)?;
        let value = CatalogDto {
            schema: 1,
            jobs: catalog.jobs.iter().map(JobDto::from_domain).collect(),
            active_versions: catalog.active_versions.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&value).map_err(|_| failure())?;
        if bytes.len() as u64 > MAX_CATALOG_BYTES {
            return Err(failure());
        }
        atomic_write(&self.root, "catalog.json", &bytes)
    }
    fn new_job_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
    fn prepare(
        &self,
        job: &TrainingJob,
        clips: &[TrainingClip],
        base: Option<&TrainingJob>,
    ) -> Result<(), TrainingError> {
        // Validate every byte before creating any durable directory.
        if clips.len() != job.clip_count
            || clips.iter().map(|clip| clip.wav.len()).sum::<usize>() != job.total_bytes
        {
            return Err(failure());
        }
        for clip in clips {
            clip.validate()?;
            validate_wav(&clip.wav)?;
        }
        let base_pair = base.map(|previous| {
            if previous.parameters.voice_id != job.parameters.voice_id || previous.id == job.id || previous.state != TrainingState::Succeeded {
                return Err(TrainingError::Invalid("续训只能使用同一音色的成功版本".into()));
            }
            self.resolve_pair(&previous.id, previous.artifacts.as_ref().ok_or_else(failure)?)
                .map_err(|_| TrainingError::Store("上一训练版本的权重缺失或损坏，无法继续训练；请检查模型文件或删除损坏版本后重试".into()))
        }).transpose()?;
        let destination = self.job_dir(&job.id)?;
        let staging = self
            .root
            .join("jobs")
            .join(format!(".{}.tmp", uuid::Uuid::new_v4()));
        fs::create_dir(&staging).map_err(|_| failure())?;
        let result = (|| {
            fs::create_dir(staging.join("clips")).map_err(|_| failure())?;
            fs::create_dir(staging.join("artifacts")).map_err(|_| failure())?;
            let mut manifest_clips = Vec::new();
            for (index, clip) in clips.iter().enumerate() {
                let relative = format!("clips/{index:03}.wav");
                write_new(&staging.join(&relative), &clip.wav)?;
                manifest_clips.push(serde_json::json!({ "path":relative, "text":clip.text, "language":clip.language }));
            }
            let text_mode = if clips.iter().all(|clip| clip.text.is_empty()) {
                "audio_only"
            } else {
                "reviewed_text"
            };
            let performance = &job.parameters.performance;
            let mut manifest = serde_json::json!({ "schema":1, "text_mode":text_mode, "id":job.id, "voice_id":job.parameters.voice_id, "name":job.parameters.name, "model_version":"v2", "sovits_epochs":job.parameters.sovits_epochs, "gpt_epochs":job.parameters.gpt_epochs, "performance": { "batch_size":performance.batch_size, "data_workers":performance.data_workers, "cpu_threads":performance.cpu_threads, "gpu_index":performance.gpu_index, "low_memory":performance.low_memory }, "fp16":true, "clips":manifest_clips, "artifacts": { "gpt":"artifacts/gpt.ckpt", "sovits":"artifacts/sovits.pth" } });
            if let (Some(previous), Some(pair)) = (base, base_pair.as_ref()) {
                fs::create_dir(staging.join("base")).map_err(|_| failure())?;
                let expected: ArtifactManifest = serde_json::from_slice(&read_bounded(
                    &self.job_dir(&previous.id)?.join("artifacts/manifest.json"),
                    4096,
                )?)
                .map_err(|_| failure())?;
                for (source, filename, key) in [
                    (&pair.gpt, "gpt.ckpt", "gpt"),
                    (&pair.sovits, "sovits.pth", "sovits"),
                ] {
                    copy_verified(
                        source,
                        &staging.join("base").join(filename),
                        expected.sha256.get(key).ok_or_else(failure)?,
                    )?;
                }
                manifest["base"] = serde_json::json!({ "job_id":previous.id, "voice_id":previous.parameters.voice_id, "gpt":"base/gpt.ckpt", "sovits":"base/sovits.pth", "sha256":expected.sha256 });
            }
            write_new(
                &staging.join("job.json"),
                &serde_json::to_vec_pretty(&manifest).map_err(|_| failure())?,
            )?;
            // The UUID directory is immutable and cannot replace an existing task.
            if fs::symlink_metadata(&destination).is_ok() {
                return Err(failure());
            }
            fs::rename(&staging, &destination).map_err(|_| failure())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }
    fn remove_unpublished(&self, id: &str) -> Result<(), TrainingError> {
        let path = self.job_dir(id)?;
        checked_directory(&path)?;
        fs::remove_dir_all(path).map_err(|_| failure())
    }
    fn remove_published(&self, id: &str) -> Result<(), TrainingError> {
        let path = self.job_dir(id)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(failure());
                }
                fs::remove_dir_all(path).map_err(|_| failure())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(failure()),
        }
    }
    fn prepared(&self, id: &str) -> Result<PreparedTrainingJob, TrainingError> {
        let work_dir = self.job_dir(id)?;
        checked_directory(&work_dir)?;
        let job_file = work_dir.join("job.json");
        read_bounded(&job_file, MAX_CATALOG_BYTES)?;
        checked_directory(&work_dir.join("clips"))?;
        checked_directory(&work_dir.join("artifacts"))?;
        Ok(PreparedTrainingJob { job_file, work_dir })
    }
    fn resolve_pair(
        &self,
        id: &str,
        pair: &ArtifactPair,
    ) -> Result<ResolvedArtifactPair, TrainingError> {
        pair.validate()?;
        let directory = self.job_dir(id)?;
        checked_directory(&directory)?;
        checked_directory(&directory.join("artifacts"))?;
        let manifest: ArtifactManifest = serde_json::from_slice(&read_bounded(
            &directory.join("artifacts/manifest.json"),
            4096,
        )?)
        .map_err(|_| failure())?;
        if manifest.job_id != id || manifest.model_version != "v2" {
            return Err(failure());
        }
        let resolve = |relative: &str, name: &str| {
            let path = directory.join(relative);
            let metadata = fs::symlink_metadata(&path).map_err(|_| failure())?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() == 0
                || metadata.len() > MAX_ARTIFACT_BYTES
            {
                return Err(failure());
            }
            let expected = manifest
                .sha256
                .get(name)
                .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or_else(failure)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let identity = (
                    metadata.dev(),
                    metadata.ino(),
                    metadata.ctime(),
                    metadata.ctime_nsec(),
                    metadata.mtime(),
                    metadata.mtime_nsec(),
                    metadata.len(),
                    expected.clone(),
                );
                let cache = self.verified.lock().map_err(|_| failure())?;
                if cache.get(&path) == Some(&identity) {
                    return Ok(path);
                }
            }
            use sha2::{Digest, Sha256};
            let mut file = fs::File::open(&path).map_err(|_| failure())?;
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 65536];
            loop {
                let read = file.read(&mut buffer).map_err(|_| failure())?;
                if read == 0 {
                    break;
                }
                hash.update(&buffer[..read]);
            }
            if format!("{:x}", hash.finalize()) != *expected {
                return Err(failure());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                self.verified.lock().map_err(|_| failure())?.insert(
                    path.clone(),
                    (
                        metadata.dev(),
                        metadata.ino(),
                        metadata.ctime(),
                        metadata.ctime_nsec(),
                        metadata.mtime(),
                        metadata.mtime_nsec(),
                        metadata.len(),
                        expected.clone(),
                    ),
                );
            }
            Ok(path)
        };
        Ok(ResolvedArtifactPair {
            gpt: resolve(&pair.gpt, "gpt")?,
            sovits: resolve(&pair.sovits, "sovits")?,
        })
    }
}
pub(super) fn validate_wav(bytes: &[u8]) -> Result<(), TrainingError> {
    let audio = crate::speech::wav::decode_wav(bytes)
        .map_err(|_| TrainingError::Invalid("训练片段必须是完整 PCM16 WAV".into()))?;
    let frames = audio.samples.len() as u64 / u64::from(audio.channels);
    if !(8000..=48000).contains(&audio.sample_rate)
        || frames < u64::from(audio.sample_rate) * 3
        || frames > u64::from(audio.sample_rate) * 10
        || !audio.samples.iter().any(|sample| *sample != 0)
    {
        return Err(TrainingError::Invalid(
            "训练片段必须是 3 至 10 秒非静音的 8 至 48 kHz WAV".into(),
        ));
    }
    Ok(())
}
fn failure() -> TrainingError {
    TrainingError::Store("训练存储不可用或文件校验失败".into())
}
fn checked_directory(path: &Path) -> Result<(), TrainingError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| failure())?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(failure())
    }
}
fn ensure_directory(path: &Path) -> Result<(), TrainingError> {
    let mut cursor = PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(failure());
        }
        cursor.push(component.as_os_str());
        match fs::symlink_metadata(&cursor) {
            Ok(_) => checked_directory(&cursor)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&cursor).map_err(|_| failure())?
            }
            Err(_) => return Err(failure()),
        }
    }
    Ok(())
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), TrainingError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| failure())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| failure())
}
fn copy_verified(source: &Path, destination: &Path, expected: &str) -> Result<(), TrainingError> {
    use sha2::{Digest, Sha256};
    let metadata = fs::symlink_metadata(source).map_err(|_| failure())?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_ARTIFACT_BYTES
    {
        return Err(failure());
    }
    let mut input = fs::File::open(source)
        .map_err(|_| failure())?
        .take(MAX_ARTIFACT_BYTES + 1);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| failure())?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let count = input.read(&mut buffer).map_err(|_| failure())?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > MAX_ARTIFACT_BYTES {
            return Err(failure());
        }
        hash.update(&buffer[..count]);
        output.write_all(&buffer[..count]).map_err(|_| failure())?;
    }
    if bytes == 0 || format!("{:x}", hash.finalize()) != expected {
        return Err(failure());
    }
    output.sync_all().map_err(|_| failure())
}
fn atomic_write(root: &Path, name: &str, bytes: &[u8]) -> Result<(), TrainingError> {
    let destination = root.join(name);
    if let Ok(metadata) = fs::symlink_metadata(&destination) {
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(failure());
        }
    }
    let temporary = root.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let result = write_new(&temporary, bytes)
        .and_then(|()| fs::rename(&temporary, &destination).map_err(|_| failure()));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, TrainingError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| failure())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > maximum {
        return Err(failure());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| failure())?
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| failure())?;
    if bytes.len() as u64 > maximum {
        return Err(failure());
    }
    Ok(bytes)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogDto {
    schema: u32,
    jobs: Vec<JobDto>,
    active_versions: BTreeMap<String, String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JobDto {
    id: String,
    name: String,
    voice_id: String,
    sovits_epochs: u16,
    gpt_epochs: u16,
    #[serde(default)]
    performance: PerformanceDto,
    state: String,
    clip_count: usize,
    total_bytes: usize,
    created_at_ms: u64,
    updated_at_ms: u64,
    progress: u8,
    message: String,
    artifacts: Option<PairDto>,
    auditioned: bool,
    #[serde(default)]
    saved: Option<bool>,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PerformanceDto {
    batch_size: u16,
    data_workers: u16,
    cpu_threads: u16,
    gpu_index: u16,
    low_memory: bool,
}
impl Default for PerformanceDto {
    fn default() -> Self {
        Self::from_domain(TrainingPerformance::default())
    }
}
impl PerformanceDto {
    fn from_domain(value: TrainingPerformance) -> Self {
        Self {
            batch_size: value.batch_size,
            data_workers: value.data_workers,
            cpu_threads: value.cpu_threads,
            gpu_index: value.gpu_index,
            low_memory: value.low_memory,
        }
    }
    fn into_domain(self) -> TrainingPerformance {
        TrainingPerformance {
            batch_size: self.batch_size,
            data_workers: self.data_workers,
            cpu_threads: self.cpu_threads,
            gpu_index: self.gpu_index,
            low_memory: self.low_memory,
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PairDto {
    model_version: String,
    gpt: String,
    sovits: String,
}
impl JobDto {
    fn from_domain(job: &TrainingJob) -> Self {
        Self {
            id: job.id.clone(),
            name: job.parameters.name.clone(),
            voice_id: job.parameters.voice_id.clone(),
            sovits_epochs: job.parameters.sovits_epochs,
            gpt_epochs: job.parameters.gpt_epochs,
            performance: PerformanceDto::from_domain(job.parameters.performance),
            state: job.state.as_str().into(),
            clip_count: job.clip_count,
            total_bytes: job.total_bytes,
            created_at_ms: job.created_at_ms,
            updated_at_ms: job.updated_at_ms,
            progress: job.progress,
            message: job.message.clone(),
            artifacts: job.artifacts.as_ref().map(|pair| PairDto {
                model_version: pair.model_version.clone(),
                gpt: pair.gpt.clone(),
                sovits: pair.sovits.clone(),
            }),
            auditioned: job.auditioned,
            saved: Some(job.saved),
        }
    }
    fn into_domain(self, active: bool) -> Result<TrainingJob, TrainingError> {
        Ok(TrainingJob {
            id: self.id,
            parameters: TrainingParameters {
                name: self.name,
                voice_id: self.voice_id,
                sovits_epochs: self.sovits_epochs,
                gpt_epochs: self.gpt_epochs,
                performance: self.performance.into_domain(),
            },
            state: TrainingState::parse(&self.state).ok_or_else(failure)?,
            clip_count: self.clip_count,
            total_bytes: self.total_bytes,
            created_at_ms: self.created_at_ms,
            updated_at_ms: self.updated_at_ms,
            progress: self.progress,
            message: self.message,
            artifacts: self.artifacts.map(|pair| ArtifactPair {
                model_version: pair.model_version,
                gpt: pair.gpt,
                sovits: pair.sovits,
            }),
            auditioned: self.auditioned,
            // Older catalogs did not record explicit saves. Only an activated
            // version proves that the user had accepted the auditioned model.
            saved: self.saved.unwrap_or(active),
        })
    }
}

#[derive(Deserialize)]
struct ArtifactManifest {
    job_id: String,
    model_version: String,
    sha256: BTreeMap<String, String>,
}
