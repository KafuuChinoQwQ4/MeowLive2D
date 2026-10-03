//! 结构化运行日志契约。摘要由受信服务端事件目录生成，禁止携带原始输入。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: RuntimeLogLevel,
    pub source: String,
    pub category: String,
    pub code: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLogListResponse {
    pub entries: Vec<RuntimeLogEntry>,
    pub storage_available: bool,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLogRequest {
    pub code: String,
}
