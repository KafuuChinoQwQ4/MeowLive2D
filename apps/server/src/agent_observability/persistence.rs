//! Agent trace 的活动快照、分段历史、恢复和保留策略。
use meowlive_protocol::agent_observability::{AgentTrace, AgentTraceStatus};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
};

const RECORD_BYTES: usize = 256 * 1024;
const SEGMENT_BYTES: u64 = 4 * 1024 * 1024;
const TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const TOTAL_RECORDS: usize = 1_000;
const SEGMENT_RECORDS: usize = 100;

pub(super) enum DiskOpenError {
    Corrupt(String),
    Unavailable(String),
}

impl DiskOpenError {
    fn corrupt(message: impl Into<String>) -> Self {
        Self::Corrupt(message.into())
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable(message.into())
    }
}

struct Segment {
    path: PathBuf,
    bytes: u64,
    records: usize,
}

pub(super) struct DiskTraceStore {
    root: PathBuf,
    segments: Vec<Segment>,
    next_index: usize,
}

impl DiskTraceStore {
    pub fn open(root: &Path) -> Result<(Self, Vec<AgentTrace>), DiskOpenError> {
        std::fs::create_dir_all(root)
            .map_err(|_| DiskOpenError::unavailable("无法建立 Agent trace 存储目录"))?;
        if !std::fs::symlink_metadata(root).is_ok_and(|metadata| metadata.is_dir()) {
            return Err(DiskOpenError::unavailable("Agent trace 存储目录无效"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| DiskOpenError::unavailable("无法保护 Agent trace 目录"))?;
        }
        let mut indexed = Vec::new();
        for entry in std::fs::read_dir(root)
            .map_err(|_| DiskOpenError::unavailable("无法读取 Agent trace 目录"))?
        {
            let entry =
                entry.map_err(|_| DiskOpenError::unavailable("无法读取 Agent trace 目录"))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(number) = name
                .strip_prefix("trace-")
                .and_then(|name| name.strip_suffix(".jsonl"))
            {
                if number.len() != 8 {
                    return Err(DiskOpenError::corrupt("Agent trace 分段名无效"));
                }
                let index = number
                    .parse::<usize>()
                    .map_err(|_| DiskOpenError::corrupt("Agent trace 分段名无效"))?;
                indexed.push((index, entry.path()));
            }
        }
        indexed.sort_by_key(|(index, _)| *index);
        let mut history = Vec::new();
        let mut segments = Vec::new();
        for (_, path) in &indexed {
            let bytes = file_length(path, SEGMENT_BYTES)?;
            let records = read_segment(path, bytes, |trace| history.push(trace))?;
            segments.push(Segment {
                path: path.clone(),
                bytes,
                records,
            });
        }
        let next_index = indexed.last().map_or(0, |(index, _)| index + 1);
        let mut store = Self {
            root: root.to_owned(),
            segments,
            next_index,
        };
        let active_path = root.join("active.json");
        if let Some(mut active) = read_json(&active_path, RECORD_BYTES as u64)? {
            validate_trace(&active, true).map_err(DiskOpenError::corrupt)?;
            if history
                .iter()
                .any(|trace| trace.summary.id == active.summary.id)
            {
                store.clear_active().map_err(DiskOpenError::unavailable)?;
                if history.len() > TOTAL_RECORDS {
                    history.drain(..history.len() - TOTAL_RECORDS);
                }
                return Ok((store, history));
            }
            super::finish_value(
                &mut active,
                AgentTraceStatus::Interrupted,
                "主服务重启，本次任务被中断",
            );
            fit_record(&mut active).map_err(DiskOpenError::corrupt)?;
            store.append(&active).map_err(DiskOpenError::unavailable)?;
            store.clear_active().map_err(DiskOpenError::unavailable)?;
            history.push(active);
        }
        if history.len() > TOTAL_RECORDS {
            history.drain(..history.len() - TOTAL_RECORDS);
        }
        Ok((store, history))
    }

    pub fn save_active(&self, trace: &AgentTrace) -> Result<(), String> {
        validate_trace(trace, true)?;
        atomic_json(&self.root.join("active.json"), trace)
    }

    pub fn clear_active(&self) -> Result<(), String> {
        let path = self.root.join("active.json");
        match std::fs::remove_file(path) {
            Ok(()) => sync_directory(&self.root).map_err(|_| "无法保存 Agent trace 目录".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("无法清理 Agent 活动 trace".into()),
        }
    }

    pub fn append(&mut self, trace: &AgentTrace) -> Result<(), String> {
        validate_trace(trace, false)?;
        let mut bytes = serde_json::to_vec(trace).map_err(|_| "无法编码 Agent trace")?;
        bytes.push(b'\n');
        if bytes.len() > RECORD_BYTES {
            return Err("Agent trace 超过长度上限".into());
        }
        let rotate = self.segments.last().is_none_or(|segment| {
            segment.bytes.saturating_add(bytes.len() as u64) > SEGMENT_BYTES
                || segment.records >= SEGMENT_RECORDS
        });
        if rotate {
            let path = self
                .root
                .join(format!("trace-{:08}.jsonl", self.next_index));
            self.next_index = self.next_index.saturating_add(1);
            let mut options = private_options();
            options.create_new(true).write(true);
            let file = options
                .open(&path)
                .map_err(|_| "无法建立 Agent trace 分段")?;
            file.sync_all().map_err(|_| "无法保存 Agent trace 分段")?;
            self.segments.push(Segment {
                path,
                bytes: 0,
                records: 0,
            });
            sync_directory(&self.root).map_err(|_| "无法保存 Agent trace 目录")?;
        }
        let segment = self.segments.last_mut().ok_or("Agent trace 分段不可用")?;
        let mut options = private_options();
        options.append(true);
        let mut file = options
            .open(&segment.path)
            .map_err(|_| "无法打开 Agent trace 分段")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "无法持久化 Agent trace")?;
        segment.bytes += bytes.len() as u64;
        segment.records += 1;
        self.enforce_retention()
    }

    fn enforce_retention(&mut self) -> Result<(), String> {
        while self.segments.len() > 1
            && (self
                .segments
                .iter()
                .map(|segment| segment.bytes)
                .sum::<u64>()
                > TOTAL_BYTES
                || self
                    .segments
                    .iter()
                    .map(|segment| segment.records)
                    .sum::<usize>()
                    > TOTAL_RECORDS)
        {
            let oldest = self.segments.remove(0);
            std::fs::remove_file(oldest.path).map_err(|_| "无法清理旧 Agent trace 分段")?;
            sync_directory(&self.root).map_err(|_| "无法保存 Agent trace 目录")?;
        }
        Ok(())
    }
}

fn validate_trace(trace: &AgentTrace, active: bool) -> Result<(), String> {
    if trace.summary.id.is_empty()
        || trace.summary.id.len() > 128
        || trace.events.len() > 16
        || trace.turns.len() > 16
        || trace.steps.len() > 256
        || (active && trace.summary.status != AgentTraceStatus::Running)
        || (!active && trace.summary.status == AgentTraceStatus::Running)
        || trace
            .steps
            .windows(2)
            .any(|steps| steps[0].sequence >= steps[1].sequence)
    {
        return Err("Agent trace 字段无效".into());
    }
    Ok(())
}

fn read_segment(
    path: &Path,
    length: u64,
    mut visitor: impl FnMut(AgentTrace),
) -> Result<usize, DiskOpenError> {
    let file =
        File::open(path).map_err(|_| DiskOpenError::unavailable("无法读取 Agent trace 分段"))?;
    let mut reader = BufReader::new(file.take(length));
    let mut count = 0;
    loop {
        let mut line = Vec::new();
        let read = reader
            .by_ref()
            .take((RECORD_BYTES + 1) as u64)
            .read_until(b'\n', &mut line)
            .map_err(|_| DiskOpenError::unavailable("无法读取 Agent trace"))?;
        if read == 0 {
            break;
        }
        if read > RECORD_BYTES || line.last() != Some(&b'\n') {
            return Err(DiskOpenError::corrupt("Agent trace 记录过长或截断"));
        }
        let trace: AgentTrace = serde_json::from_slice(&line)
            .map_err(|_| DiskOpenError::corrupt("Agent trace 内容损坏"))?;
        validate_trace(&trace, false).map_err(DiskOpenError::corrupt)?;
        visitor(trace);
        count += 1;
    }
    Ok(count)
}

fn atomic_json(path: &Path, value: &AgentTrace) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|_| "无法编码 Agent 活动 trace")?;
    if bytes.len() > RECORD_BYTES {
        return Err("Agent 活动 trace 超过长度上限".into());
    }
    let temporary = path.with_extension("tmp");
    let result: Result<(), String> = (|| {
        let mut options = private_options();
        options.create(true).truncate(true).write(true);
        let mut file = options
            .open(&temporary)
            .map_err(|_| "无法建立 Agent 活动 trace")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "无法保存 Agent 活动 trace")?;
        std::fs::rename(&temporary, path).map_err(|_| "无法替换 Agent 活动 trace")?;
        sync_directory(path.parent().ok_or("Agent trace 目录无效")?)
            .map_err(|_| "无法保存 Agent trace 目录".into())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

fn read_json(path: &Path, maximum: u64) -> Result<Option<AgentTrace>, DiskOpenError> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(DiskOpenError::unavailable("无法读取 Agent trace 文件")),
        Ok(_) => {}
    }
    file_length(path, maximum)?;
    let bytes =
        std::fs::read(path).map_err(|_| DiskOpenError::unavailable("无法读取 Agent trace 文件"))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| DiskOpenError::corrupt("Agent trace 文件内容损坏"))
}

fn file_length(path: &Path, maximum: u64) -> Result<u64, DiskOpenError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| DiskOpenError::unavailable("无法读取 Agent trace 文件"))?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(DiskOpenError::corrupt("Agent trace 文件类型或长度无效"));
    }
    Ok(metadata.len())
}

pub(super) fn fit_record(trace: &mut AgentTrace) -> Result<(), String> {
    while serde_json::to_vec(trace)
        .map_err(|_| "无法编码 Agent trace")?
        .len()
        .saturating_add(1)
        > RECORD_BYTES
    {
        trace.summary.truncated = true;
        // Sources dominate record size; retain operation and terminal facts first.
        if let Some(step) = trace.steps.iter_mut().find(|step| !step.sources.is_empty()) {
            step.sources.clear();
        } else if trace.steps.len() > 1 {
            trace.steps.remove(0);
        } else if trace.events.pop().is_none()
            && trace.turns.pop().is_none()
            && trace.steps.pop().is_none()
        {
            return Err("Agent trace 超过长度上限".into());
        }
    }
    Ok(())
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
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
