//! 有界结构化日志。仅枚举事件可写入；磁盘恢复也重新生成固定摘要。
use meowlive_protocol::log::{RuntimeLogEntry, RuntimeLogLevel as Level, RuntimeLogListResponse};
use std::{
    collections::{HashMap, VecDeque},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant, SystemTime},
};
const MAX_ENTRIES: usize = 5000;
const SEGMENT_ENTRIES: usize = 2500;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogEvent {
    ServerStarted,
    ServerStopping,
    ServerFailed,
    BrowserError,
    UnhandledRejection,
    RequestFailed,
    HttpClientError,
    HttpServerError,
    OperationCompleted,
    LlmCompleted,
    LlmFailed,
    LlmCancelled,
    LiveConnected,
    LiveRetrying,
    LiveFailed,
    LiveDisconnected,
    SpeechReady,
    SpeechFailed,
    AgentFailed,
    MemoryFailed,
    BridgeConnected,
    BridgeDisconnected,
    ExecutionCompleted,
    ExecutionFailed,
    ExecutionCancelled,
    ResourceCompleted,
    ResourceFailed,
    ObsCompleted,
    ObsFailed,
    ReceiptFailed,
    GraphFailed,
    TrainingCompleted,
    TrainingFailed,
    TrainingCancelled,
}
impl LogEvent {
    fn fields(
        self,
    ) -> (
        Level,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        use LogEvent::*;
        match self {
            BridgeConnected => (
                Level::Info,
                "bridge",
                "execution",
                "bridge_connected",
                "桌面执行端已连接",
            ),
            BridgeDisconnected => (
                Level::Warn,
                "bridge",
                "execution",
                "bridge_disconnected",
                "桌面执行端已断开",
            ),
            ExecutionCompleted => (
                Level::Info,
                "bridge",
                "execution",
                "execution_completed",
                "桌面播放已完成",
            ),
            ExecutionFailed => (
                Level::Error,
                "bridge",
                "execution",
                "execution_failed",
                "桌面播放失败",
            ),
            ExecutionCancelled => (
                Level::Warn,
                "bridge",
                "execution",
                "execution_cancelled",
                "桌面播放已取消",
            ),
            ResourceCompleted => (
                Level::Info,
                "bridge",
                "resources",
                "resource_completed",
                "桌面资源操作已完成",
            ),
            ResourceFailed => (
                Level::Error,
                "bridge",
                "resources",
                "resource_failed",
                "桌面资源操作失败或结果未知",
            ),
            ObsCompleted => (
                Level::Info,
                "bridge",
                "obs",
                "obs_completed",
                "OBS 操作已完成",
            ),
            ObsFailed => (
                Level::Error,
                "bridge",
                "obs",
                "obs_failed",
                "OBS 操作失败或结果未知",
            ),
            ReceiptFailed => (
                Level::Error,
                "server",
                "companionship",
                "receipt_failed",
                "完成回执存储失败，将重试",
            ),
            GraphFailed => (
                Level::Error,
                "server",
                "graph",
                "graph_failed",
                "关系图同步失败，将重试",
            ),
            TrainingCompleted => (
                Level::Info,
                "server",
                "training",
                "training_completed",
                "训练任务已完成",
            ),
            TrainingFailed => (
                Level::Error,
                "server",
                "training",
                "training_failed",
                "训练任务失败",
            ),
            TrainingCancelled => (
                Level::Warn,
                "server",
                "training",
                "training_cancelled",
                "训练任务已取消",
            ),
            ServerStarted => (
                Level::Info,
                "server",
                "lifecycle",
                "server_started",
                "主服务已启动",
            ),
            ServerStopping => (
                Level::Info,
                "server",
                "lifecycle",
                "server_stopping",
                "主服务正在停止",
            ),
            ServerFailed => (
                Level::Error,
                "server",
                "lifecycle",
                "server_failed",
                "主服务异常退出",
            ),
            BrowserError => (
                Level::Error,
                "browser",
                "runtime",
                "browser_error",
                "浏览器运行错误",
            ),
            UnhandledRejection => (
                Level::Error,
                "browser",
                "runtime",
                "unhandled_rejection",
                "浏览器异步任务失败",
            ),
            RequestFailed => (
                Level::Warn,
                "browser",
                "http",
                "request_failed",
                "浏览器请求失败",
            ),
            HttpClientError => (
                Level::Warn,
                "server",
                "http",
                "http_client_error",
                "请求被拒绝，请检查输入、认证或当前状态",
            ),
            HttpServerError => (
                Level::Error,
                "server",
                "http",
                "http_server_error",
                "服务未能完成请求",
            ),
            OperationCompleted => (
                Level::Info,
                "server",
                "http",
                "operation_completed",
                "管理操作已完成",
            ),
            LlmCompleted => (
                Level::Info,
                "server",
                "llm",
                "llm_completed",
                "模型调用已完成",
            ),
            LlmFailed => (Level::Error, "server", "llm", "llm_failed", "模型调用失败"),
            LlmCancelled => (
                Level::Debug,
                "server",
                "llm",
                "llm_cancelled",
                "模型调用已取消",
            ),
            LiveConnected => (
                Level::Info,
                "server",
                "live",
                "live_connected",
                "直播事件连接已建立",
            ),
            LiveRetrying => (
                Level::Warn,
                "server",
                "live",
                "live_retrying",
                "直播连接异常，正在重连",
            ),
            LiveFailed => (
                Level::Error,
                "server",
                "live",
                "live_failed",
                "直播连接失败",
            ),
            LiveDisconnected => (
                Level::Info,
                "server",
                "live",
                "live_disconnected",
                "直播连接已断开",
            ),
            SpeechReady => (
                Level::Info,
                "server",
                "speech",
                "speech_ready",
                "语音合成完成",
            ),
            SpeechFailed => (
                Level::Error,
                "server",
                "speech",
                "speech_failed",
                "语音合成失败或结果无效",
            ),
            AgentFailed => (
                Level::Error,
                "server",
                "agent",
                "agent_failed",
                "Agent 决策处理失败",
            ),
            MemoryFailed => (
                Level::Error,
                "server",
                "memory",
                "memory_failed",
                "记忆后台任务失败，将按任务策略重试",
            ),
        }
    }
    fn from_code(code: &str) -> Option<Self> {
        use LogEvent::*;
        [
            ServerStarted,
            ServerStopping,
            ServerFailed,
            BrowserError,
            UnhandledRejection,
            RequestFailed,
            HttpClientError,
            HttpServerError,
            OperationCompleted,
            LlmCompleted,
            LlmFailed,
            LlmCancelled,
            LiveConnected,
            LiveRetrying,
            LiveFailed,
            LiveDisconnected,
            SpeechReady,
            SpeechFailed,
            AgentFailed,
            MemoryFailed,
            BridgeConnected,
            BridgeDisconnected,
            ExecutionCompleted,
            ExecutionFailed,
            ExecutionCancelled,
            ResourceCompleted,
            ResourceFailed,
            ObsCompleted,
            ObsFailed,
            ReceiptFailed,
            GraphFailed,
            TrainingCompleted,
            TrainingFailed,
            TrainingCancelled,
        ]
        .into_iter()
        .find(|v| v.fields().3 == code)
    }
}

#[derive(Clone, Copy)]
pub enum LogCategory {
    Http,
    Auth,
    Agent,
    Llm,
    Live,
    Speech,
    Training,
    Resources,
    Memory,
    Viewers,
    Obs,
}
impl LogCategory {
    pub fn from_path(path: &str) -> Self {
        if path.starts_with("/api/admin/session") {
            Self::Auth
        } else if path.starts_with("/api/agent") || path == "/api/events" {
            Self::Agent
        } else if path.starts_with("/api/llm") {
            Self::Llm
        } else if path.starts_with("/api/live") {
            Self::Live
        } else if path.starts_with("/api/training") || path.starts_with("/api/runtime") {
            Self::Training
        } else if path.starts_with("/api/resources")
            || path.starts_with("/api/voices")
            || path.starts_with("/api/characters")
            || path.starts_with("/api/desktop/resources")
        {
            Self::Resources
        } else if path.starts_with("/api/admin/memories") || path.contains("/memories") {
            Self::Memory
        } else if path.starts_with("/api/admin/") {
            Self::Viewers
        } else if path.starts_with("/api/obs") {
            Self::Obs
        } else if path == "/api/speech" || path == "/api/stop" {
            Self::Speech
        } else {
            Self::Http
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Auth => "auth",
            Self::Agent => "agent",
            Self::Llm => "llm",
            Self::Live => "live",
            Self::Speech => "speech",
            Self::Training => "training",
            Self::Resources => "resources",
            Self::Memory => "memory",
            Self::Viewers => "viewers",
            Self::Obs => "obs",
        }
    }
}
struct Inner {
    entries: VecDeque<RuntimeLogEntry>,
    available: bool,
    truncated: bool,
    disk_count: usize,
    repeated: HashMap<LogEvent, Instant>,
}
pub struct RuntimeLogStore {
    inner: Mutex<Inner>,
    path: Option<PathBuf>,
}
impl RuntimeLogStore {
    pub fn memory() -> Self {
        Self {
            inner: Mutex::new(Inner {
                entries: VecDeque::new(),
                available: false,
                truncated: false,
                disk_count: 0,
                repeated: HashMap::new(),
            }),
            path: None,
        }
    }
    /// Failure preserves process availability and is visible in the query response.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut store = Self::memory();
        let mut entries = VecDeque::new();
        let mut oversized = false;
        let loaded = (|| -> std::io::Result<usize> {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            for file_path in [path.with_extension("previous.jsonl"), path.clone()] {
                let file = match File::open(&file_path) {
                    Ok(f) => f,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(e) => return Err(e),
                };
                if file.metadata()?.len() > MAX_FILE_BYTES {
                    oversized = true;
                    return Err(std::io::Error::other("log segment too large"));
                }
                for line in BufReader::new(file.take(MAX_FILE_BYTES))
                    .lines()
                    .take(SEGMENT_ENTRIES)
                {
                    let line = line?;
                    if line.len() > 2048 {
                        continue;
                    }
                    let Ok(mut entry) = serde_json::from_str::<RuntimeLogEntry>(&line) else {
                        continue;
                    };
                    let Some(event) = LogEvent::from_code(&entry.code) else {
                        continue;
                    };
                    if uuid::Uuid::parse_str(&entry.id).is_err()
                        || chrono::DateTime::parse_from_rfc3339(&entry.timestamp).is_err()
                    {
                        continue;
                    }
                    let (level, source, category, _, summary) = event.fields();
                    entry.level = level;
                    entry.source = source.into();
                    entry.summary = summary.into();
                    if !matches!(
                        event,
                        LogEvent::HttpClientError
                            | LogEvent::HttpServerError
                            | LogEvent::OperationCompleted
                    ) || ![
                        "http",
                        "auth",
                        "agent",
                        "llm",
                        "live",
                        "speech",
                        "training",
                        "resources",
                        "memory",
                        "viewers",
                        "obs",
                    ]
                    .contains(&entry.category.as_str())
                    {
                        entry.category = category.into();
                    }
                    entries.push_back(entry);
                }
            }
            let count = if path.exists() {
                BufReader::new(File::open(&path)?)
                    .lines()
                    .take(SEGMENT_ENTRIES)
                    .count()
            } else {
                0
            };
            let mut file = append_file(&path)?;
            if file.metadata()?.len() > 0 {
                file.seek(SeekFrom::End(-1))?;
                let mut tail = [0];
                file.read_exact(&mut tail)?;
                if tail[0] != b'\n' {
                    file.write_all(b"\n")?;
                }
            }
            Ok(count)
        })();
        let inner = store.inner.get_mut().unwrap();
        inner.entries = entries;
        inner.available = loaded.is_ok();
        inner.disk_count = loaded.unwrap_or(SEGMENT_ENTRIES);
        inner.truncated = path.with_extension("previous.jsonl").exists();
        store.path = (!oversized).then_some(path);
        store
    }
    pub fn record_code(&self, code: &str) -> Result<RuntimeLogEntry, &'static str> {
        let event = match code {
            "browser_error" => LogEvent::BrowserError,
            "unhandled_rejection" => LogEvent::UnhandledRejection,
            "request_failed" => LogEvent::RequestFailed,
            _ => return Err("unsupported_log_code"),
        };
        Ok(self.record(event))
    }
    /// Suppress repeated background failures for 30 seconds per fixed event kind.
    pub fn record_throttled(&self, event: LogEvent) {
        let mut inner = self.inner.lock().unwrap();
        let now = Instant::now();
        if inner
            .repeated
            .get(&event)
            .is_some_and(|last| now.duration_since(*last) < Duration::from_secs(30))
        {
            return;
        }
        inner.repeated.insert(event, now);
        drop(inner);
        self.record(event);
    }
    pub fn record(&self, event: LogEvent) -> RuntimeLogEntry {
        self.record_in(event, None)
    }
    pub(crate) fn record_in(
        &self,
        event: LogEvent,
        category: Option<LogCategory>,
    ) -> RuntimeLogEntry {
        let (level, source, default_category, code, summary) = event.fields();
        let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
        let entry = RuntimeLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level,
            source: source.into(),
            category: category
                .map(LogCategory::name)
                .unwrap_or(default_category)
                .into(),
            code: code.into(),
            summary: summary.into(),
        };
        let mut inner = self.inner.lock().unwrap();
        inner.entries.push_back(entry.clone());
        if inner.entries.len() > MAX_ENTRIES {
            inner.entries.pop_front();
            inner.truncated = true;
        }
        if let Some(path) = &self.path {
            inner.available = (|| -> std::io::Result<()> {
                if inner.disk_count >= SEGMENT_ENTRIES {
                    let previous = path.with_extension("previous.jsonl");
                    if previous.exists() {
                        fs::remove_file(&previous)?;
                    }
                    if path.exists() {
                        fs::rename(path, previous)?;
                    }
                    inner.disk_count = 0;
                    inner.truncated = true;
                }
                let mut file = append_file(path)?;
                let mut data = serde_json::to_vec(&entry)?;
                data.push(b'\n');
                file.write_all(&data)?;
                inner.disk_count += 1;
                Ok(())
            })()
            .is_ok();
        }
        entry
    }
    pub fn list(
        &self,
        level: Option<&str>,
        source: Option<&str>,
        category: Option<&str>,
        query: Option<&str>,
        limit: usize,
    ) -> RuntimeLogListResponse {
        let inner = self.inner.lock().unwrap();
        let limit = limit.clamp(1, MAX_ENTRIES);
        let query = query.map(str::to_lowercase);
        let entries: Vec<_> = inner
            .entries
            .iter()
            .rev()
            .filter(|e| {
                level.is_none_or(|v| {
                    v.eq_ignore_ascii_case(match e.level {
                        Level::Debug => "debug",
                        Level::Info => "info",
                        Level::Warn => "warn",
                        Level::Error => "error",
                    })
                }) && source.is_none_or(|v| e.source.eq_ignore_ascii_case(v))
                    && category.is_none_or(|v| e.category.eq_ignore_ascii_case(v))
                    && query.as_deref().is_none_or(|v| {
                        e.code.to_lowercase().contains(v)
                            || e.summary.to_lowercase().contains(v)
                            || e.source.to_lowercase().contains(v)
                            || e.category.to_lowercase().contains(v)
                    })
            })
            .take(limit + 1)
            .cloned()
            .collect();
        let truncated = inner.truncated || entries.len() > limit;
        RuntimeLogListResponse {
            entries: entries.into_iter().take(limit).collect(),
            storage_available: inner.available,
            truncated,
        }
    }
}

fn append_file(path: &std::path::Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).append(true).read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
