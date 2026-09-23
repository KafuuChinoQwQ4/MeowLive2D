//! Agent 调度、模型 turn、工具和播放链路的管理员观察契约。
use crate::{agent::AgentPhase, llm_runtime::LlmTokenUsage};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerBlockReason {
    Ready,
    Paused,
    ModelUnavailable,
    BridgeDisconnected,
    ResourceChanging,
    GpuBusy,
    SynthesizerBusy,
    ReceiptBacklog,
    PlaybackBusy,
    DecisionInFlight,
    SpeechInFlight,
    Cooldown,
    CompletionBufferFull,
    WorkIdExhausted,
    NoEligibleEvents,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentSchedulerSnapshot {
    pub ready: bool,
    pub phase: AgentPhase,
    pub block_reason: AgentSchedulerBlockReason,
    pub message: String,
    #[ts(type = "number | null")]
    pub remaining_ms: Option<u64>,
    pub pending_events: u32,
    pub deciding_events: u32,
    pub active_speeches: u32,
    #[ts(type = "number")]
    pub updated_at_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentTraceStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentTraceStepKind {
    Scheduled,
    ContextReady,
    TurnStarted,
    FirstToken,
    TurnFinished,
    ToolStarted,
    ToolFinished,
    DecisionReceived,
    ValidationFinished,
    SpeechQueued,
    SpeechSynthesizing,
    SpeechReady,
    SpeechPlaying,
    TraceFinished,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentTraceStepStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentTraceStep {
    #[ts(type = "number")]
    pub sequence: u64,
    #[ts(type = "number")]
    pub occurred_at_ms: u64,
    pub kind: AgentTraceStepKind,
    pub status: AgentTraceStepStatus,
    pub message: String,
    pub turn_id: Option<String>,
    pub tool_name: Option<String>,
    pub speech_id: Option<String>,
    #[ts(type = "number | null")]
    pub elapsed_ms: Option<u64>,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentTraceEvent {
    pub id: String,
    pub kind: String,
    pub viewer: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentTurn {
    pub id: String,
    pub index: u32,
    pub tool_round: u32,
    pub retry_attempt: u32,
    pub provider: String,
    pub api_format: String,
    pub model: String,
    pub status: AgentTraceStatus,
    #[ts(type = "number")]
    pub started_at_ms: u64,
    #[ts(type = "number | null")]
    pub first_token_ms: Option<u64>,
    #[ts(type = "number | null")]
    pub finished_at_ms: Option<u64>,
    #[ts(type = "number")]
    pub latency_ms: u64,
    pub usage: LlmTokenUsage,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentTraceSummary {
    pub id: String,
    pub status: AgentTraceStatus,
    pub trigger: String,
    #[ts(type = "number")]
    pub started_at_ms: u64,
    #[ts(type = "number")]
    pub updated_at_ms: u64,
    #[ts(type = "number | null")]
    pub finished_at_ms: Option<u64>,
    pub event_count: u32,
    pub turn_count: u32,
    pub tool_count: u32,
    pub speech_id: Option<String>,
    pub result: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentTrace {
    pub summary: AgentTraceSummary,
    pub events: Vec<AgentTraceEvent>,
    pub turns: Vec<AgentTurn>,
    pub steps: Vec<AgentTraceStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentTraceList {
    pub traces: Vec<AgentTraceSummary>,
    pub storage_available: bool,
    pub truncated: bool,
}
