//! 私有设置与 pending 原子替换，调用历史分段追加；读取限制单条与单段大小。
use super::settings;
use meowlive_protocol::llm_runtime::{AgentRuntimeSettings, LlmUsageRecord};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
};

const SEGMENT_BYTES: u64 = 4 * 1024 * 1024;
const RECORD_BYTES: usize = 16 * 1024;
const SETTINGS_BYTES: u64 = 512 * 1024;
const PENDING_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedSettings {
    schema: u8,
    pub settings: AgentRuntimeSettings,
    pub search_api_key: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    schema: u8,
    records: Vec<LlmUsageRecord>,
}

pub(super) struct DiskLedger {
    root: PathBuf,
    segments: Vec<PathBuf>,
}
pub(super) type SegmentSnapshot = Vec<(PathBuf, u64)>;
type OpenResult = (
    DiskLedger,
    Option<SavedSettings>,
    BTreeMap<String, LlmUsageRecord>,
);

impl DiskLedger {
    pub fn open(root: &Path) -> Result<OpenResult, String> {
        std::fs::create_dir_all(root).map_err(|_| "无法建立 Agent 运行存储目录")?;
        if !std::fs::symlink_metadata(root).is_ok_and(|metadata| metadata.is_dir()) {
            return Err("Agent 运行存储目录无效".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "无法保护 Agent 运行目录")?;
        }
        let saved: Option<SavedSettings> = read_json(&root.join("settings.json"), SETTINGS_BYTES)?;
        if let Some(saved) = &saved {
            if saved.schema != 1 {
                return Err("Agent 运行设置版本无效".into());
            }
            settings::validate(&saved.settings)?;
            settings::validate_key(&saved.search_api_key)?;
        }
        let pending: Option<Pending> = read_json(&root.join("pending.json"), PENDING_BYTES)?;
        let mut records = BTreeMap::new();
        if let Some(pending) = pending {
            if pending.schema != 1 || pending.records.len() > 128 {
                return Err("Agent 未完成调用账本无效".into());
            }
            for record in pending.records {
                validate_record(&record)?;
                if record.status != "running" || records.insert(record.id.clone(), record).is_some()
                {
                    return Err("Agent 未完成调用账本无效".into());
                }
            }
        }
        let mut indexed = Vec::new();
        for entry in std::fs::read_dir(root).map_err(|_| "无法读取 Agent 运行目录")? {
            let entry = entry.map_err(|_| "无法读取 Agent 运行目录")?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(number) = name
                .strip_prefix("usage-")
                .and_then(|name| name.strip_suffix(".jsonl"))
            {
                let index = number
                    .parse::<usize>()
                    .map_err(|_| "Agent 调用账本分段名无效")?;
                if number.len() != 8 {
                    return Err("Agent 调用账本分段名无效".into());
                }
                indexed.push((index, entry.path()));
            }
        }
        indexed.sort_by_key(|(index, _)| *index);
        if indexed
            .iter()
            .enumerate()
            .any(|(expected, (index, _))| expected != *index)
        {
            return Err("Agent 调用账本分段缺失".into());
        }
        Ok((
            Self {
                root: root.to_owned(),
                segments: indexed.into_iter().map(|(_, path)| path).collect(),
            },
            saved,
            records,
        ))
    }
    pub fn save_settings(
        &self,
        settings: &AgentRuntimeSettings,
        key: &Option<String>,
    ) -> Result<(), String> {
        atomic_json(
            &self.root.join("settings.json"),
            &SavedSettings {
                schema: 1,
                settings: settings.clone(),
                search_api_key: key.clone(),
            },
            SETTINGS_BYTES,
        )
    }
    pub fn save_pending(&self, records: &BTreeMap<String, LlmUsageRecord>) -> Result<(), String> {
        atomic_json(
            &self.root.join("pending.json"),
            &Pending {
                schema: 1,
                records: records.values().cloned().collect(),
            },
            PENDING_BYTES,
        )
    }
    pub fn append(&mut self, record: &LlmUsageRecord) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(record).map_err(|_| "无法编码 Agent 调用记录")?;
        bytes.push(b'\n');
        if bytes.len() > RECORD_BYTES {
            return Err("Agent 调用记录超过长度上限".into());
        }
        let rotate = match self.segments.last() {
            None => true,
            Some(path) => {
                metadata_len(path, SEGMENT_BYTES)?.saturating_add(bytes.len() as u64)
                    > SEGMENT_BYTES
            }
        };
        if rotate {
            let path = self
                .root
                .join(format!("usage-{:08}.jsonl", self.segments.len()));
            let mut options = private_options();
            options.create_new(true);
            let file = options
                .open(&path)
                .map_err(|_| "无法建立 Agent 调用账本分段")?;
            file.sync_all().map_err(|_| "无法保存 Agent 调用账本分段")?;
            self.segments.push(path);
            sync_directory(&self.root).map_err(|_| "无法保存 Agent 调用账本目录")?;
        }
        let path = self.segments.last().ok_or("Agent 调用账本分段不可用")?;
        metadata_len(path, SEGMENT_BYTES)?;
        let mut options = private_options();
        options.append(true);
        let mut file = options
            .open(path)
            .map_err(|_| "无法打开 Agent 调用账本分段")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "无法持久化 Agent 调用记录".into())
    }
    pub fn segments(&self) -> Result<SegmentSnapshot, String> {
        self.segments
            .iter()
            .map(|path| Ok((path.clone(), metadata_len(path, SEGMENT_BYTES)?)))
            .collect()
    }
    pub fn visit(
        &self,
        visitor: impl FnMut(LlmUsageRecord) -> Result<(), String>,
    ) -> Result<(), String> {
        visit_segments(self.segments()?, visitor)
    }
}

pub(super) fn visit_segments(
    segments: SegmentSnapshot,
    mut visitor: impl FnMut(LlmUsageRecord) -> Result<(), String>,
) -> Result<(), String> {
    for (path, length) in segments {
        if metadata_len(&path, SEGMENT_BYTES)? < length {
            return Err("Agent 调用账本长度异常".into());
        }
        let file = File::open(&path).map_err(|_| "无法读取 Agent 调用账本")?;
        let mut reader = BufReader::new(file.take(length));
        loop {
            let mut line = Vec::new();
            let read = reader
                .by_ref()
                .take((RECORD_BYTES + 1) as u64)
                .read_until(b'\n', &mut line)
                .map_err(|_| "无法读取 Agent 调用记录")?;
            if read == 0 {
                break;
            }
            if read > RECORD_BYTES || line.last() != Some(&b'\n') {
                return Err("Agent 调用账本记录过长或截断".into());
            }
            let record: LlmUsageRecord =
                serde_json::from_slice(&line).map_err(|_| "Agent 调用账本内容损坏")?;
            validate_record(&record)?;
            if record.status == "running" {
                return Err("Agent 历史调用账本状态无效".into());
            }
            visitor(record)?;
        }
    }
    Ok(())
}
fn validate_record(record: &LlmUsageRecord) -> Result<(), String> {
    if record.id.is_empty()
        || record.id.len() > 128
        || record.provider.len() > 64
        || record.model.len() > 128
        || record.api_format.len() > 64
        || record.operation != "agent"
        || !matches!(
            record.status.as_str(),
            "running" | "completed" | "failed" | "cancelled" | "interrupted"
        )
        || (!record.base_url.is_empty() && settings::validate_url(&record.base_url).is_err())
    {
        return Err("Agent 调用记录字段无效".into());
    }
    Ok(())
}
fn metadata_len(path: &Path, maximum: u64) -> Result<u64, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "无法读取 Agent 运行文件")?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err("Agent 运行文件类型或长度无效".into());
    }
    Ok(metadata.len())
}
fn read_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    maximum: u64,
) -> Result<Option<T>, String> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("无法读取 Agent 运行文件".into()),
        Ok(_) => {}
    }
    metadata_len(path, maximum)?;
    let bytes = std::fs::read(path).map_err(|_| "无法读取 Agent 运行文件")?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "Agent 运行文件内容损坏".into())
}
fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}
fn atomic_json(path: &Path, value: &impl Serialize, maximum: u64) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|_| "无法编码 Agent 运行设置")?;
    if bytes.len() as u64 > maximum {
        return Err("Agent 运行文件超过长度上限".into());
    }
    let parent = path.parent().ok_or("Agent 运行文件目录无效")?;
    let temporary = parent.join(format!(".runtime-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> std::io::Result<()> {
        let mut options = private_options();
        options.create_new(true);
        let mut file = options.open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
        return Err("无法保存 Agent 运行文件".into());
    }
    Ok(())
}
fn sync_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
