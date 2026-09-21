//! 模型决策接口，仅传递领域事件和已完成的对话，不依赖具体传输协议。
use meowlive_domain::event::LiveEvent;
use std::{fmt, future::Future, pin::Pin};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationTurn {
    pub user: String,
    pub assistant: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionRequest {
    pub persona: String,
    pub topic: String,
    pub events: Vec<LiveEvent>,
    pub history: Vec<ConversationTurn>,
    /// Confirmed viewer context, untrusted data; contains no internal scores.
    pub memory_context: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentDecision {
    pub reply_to: Vec<String>,
    pub text: Option<String>,
    pub topic: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlmError {
    pub message: String,
    pub retryable: bool,
}
impl LlmError {
    pub fn new(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            message: message.into(),
            retryable,
        }
    }
}
impl fmt::Display for LlmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl std::error::Error for LlmError {}

pub type DecisionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AgentDecision, LlmError>> + Send + 'a>>;
pub trait LanguageModel: Send + Sync {
    fn decide(&self, request: DecisionRequest) -> DecisionFuture<'_>;

    fn turn(
        &self,
        request: DecisionRequest,
        _options: super::llm_runtime::ModelOptions,
    ) -> super::llm_runtime::ModelTurnFuture<'_> {
        Box::pin(async move {
            self.decide(request)
                .await
                .map(|decision| super::llm_runtime::ModelTurn {
                    decision: Some(decision),
                    tool_calls: Vec::new(),
                    continuation: None,
                    usage: super::llm_runtime::TokenUsage::default(),
                    finish_reason: "stop".into(),
                })
        })
    }
}
