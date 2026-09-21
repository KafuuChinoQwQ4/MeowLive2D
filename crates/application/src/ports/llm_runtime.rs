//! 厂商无关的单次模型调用、工具交换和用量；JSON 仅在边界作为不透明字符串传递。
use super::llm::{AgentDecision, LlmError};
use std::{future::Future, pin::Pin, sync::Arc};

/// input 包含缓存读/写；output 包含 reasoning。None 表示厂商未报告，不能当成零。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolResult {
    pub call_id: String,
    pub name: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExchange {
    /// 原生 assistant 输出，仅回传给同一厂商；包括工具 ID 和必要的签名。
    pub continuation: String,
    pub results: Vec<ToolResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelEvent {
    FirstToken,
    OutputProgress { characters: u64 },
    Usage(TokenUsage),
}

pub trait ModelObserver: Send + Sync {
    fn on_event(&self, event: ModelEvent);
}

#[derive(Clone, Default)]
pub struct ModelOptions {
    pub reasoning_effort: super::reasoning::ReasoningEffort,
    /// 显式厂商身份；空值与 custom 仅推断已知的具体模型 ID。
    pub reasoning_provider: String,
    /// Keep definitions and cached prefixes stable while forcing a final answer.
    pub tool_choice_none: bool,
    pub tools: Vec<ToolDefinition>,
    pub exchanges: Vec<ToolExchange>,
    pub cache_enabled: bool,
    pub stream: bool,
    /// 动态环境数据放在用户上下文，不能破坏系统缓存前缀。
    pub environment: Option<String>,
    pub observer: Option<Arc<dyn ModelObserver>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelTurn {
    pub decision: Option<AgentDecision>,
    pub tool_calls: Vec<ToolCall>,
    pub continuation: Option<String>,
    pub usage: TokenUsage,
    /// stop / tool_calls；非正常结束通过 LlmError 报告。
    pub finish_reason: String,
}

pub type ModelTurnFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ModelTurn, LlmError>> + Send + 'a>>;
