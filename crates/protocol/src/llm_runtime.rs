//! Agent 工具运行配置、调用计量与实时阶段查询；密钥只接受写入。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, TS)]
#[serde(deny_unknown_fields)]
pub struct LlmPrice {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub input_usd_per_million: f64,
    pub output_usd_per_million: f64,
    pub cache_read_usd_per_million: f64,
    pub cache_write_usd_per_million: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentRuntimeSettings {
    pub cache_enabled: bool,
    pub streaming: bool,
    pub tools_enabled: bool,
    pub environment_enabled: bool,
    pub web_search_enabled: bool,
    /// brave / searxng
    pub search_provider: String,
    pub search_endpoint: String,
    pub max_tool_rounds: u32,
    pub tool_timeout_seconds: u32,
    pub prices: Vec<LlmPrice>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, TS)]
pub struct AgentRuntimeSettingsSnapshot {
    pub settings: AgentRuntimeSettings,
    pub search_key_configured: bool,
    pub storage_available: bool,
}

#[derive(Clone, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentRuntimeSettingsRequest {
    pub settings: AgentRuntimeSettings,
    pub search_api_key: Option<String>,
    pub clear_search_api_key: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct LlmTokenUsage {
    #[ts(type = "number | null")]
    pub input_tokens: Option<u64>,
    #[ts(type = "number | null")]
    pub output_tokens: Option<u64>,
    #[ts(type = "number | null")]
    pub cache_read_tokens: Option<u64>,
    #[ts(type = "number | null")]
    pub cache_write_tokens: Option<u64>,
    #[ts(type = "number | null")]
    pub reasoning_tokens: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct LlmUsageRecord {
    pub id: String,
    #[ts(type = "number")]
    pub started_at_ms: u64,
    pub provider: String,
    pub api_format: String,
    pub base_url: String,
    pub model: String,
    pub operation: String,
    /// running / completed / failed / cancelled / interrupted
    pub status: String,
    #[ts(type = "number")]
    pub latency_ms: u64,
    #[ts(type = "number | null")]
    pub first_token_ms: Option<u64>,
    pub usage: LlmTokenUsage,
    /// 微美元，null 表示未报告用量或未配置单价。
    #[ts(type = "number | null")]
    pub estimated_cost_microusd: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct LlmUsageTotals {
    #[ts(type = "number")]
    pub calls: u64,
    #[ts(type = "number")]
    pub input_tokens: u64,
    #[ts(type = "number")]
    pub output_tokens: u64,
    #[ts(type = "number")]
    pub cache_read_tokens: u64,
    #[ts(type = "number")]
    pub cache_write_tokens: u64,
    #[ts(type = "number")]
    pub reasoning_tokens: u64,
    #[ts(type = "number")]
    pub estimated_cost_microusd: u64,
    #[ts(type = "number")]
    pub unpriced_calls: u64,
    #[ts(type = "number")]
    pub unknown_usage_calls: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct LlmUsageGroup {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub totals: LlmUsageTotals,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct LlmUsageSnapshot {
    pub totals: LlmUsageTotals,
    pub groups: Vec<LlmUsageGroup>,
    pub records: Vec<LlmUsageRecord>,
    pub storage_available: bool,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct AgentToolActivity {
    pub name: String,
    /// running / completed / failed
    pub status: String,
    #[ts(type = "number")]
    pub elapsed_ms: u64,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct AgentActivitySnapshot {
    pub run_id: Option<String>,
    /// idle / thinking / receiving / tool / completed / failed / cancelled
    pub phase: String,
    #[ts(type = "number | null")]
    pub started_at_ms: Option<u64>,
    #[ts(type = "number")]
    pub updated_at_ms: u64,
    #[ts(type = "number")]
    pub output_characters: u64,
    pub tool_round: u32,
    pub tools: Vec<AgentToolActivity>,
    pub message: String,
}
