//! 厂商无关的推理意愿顺序；Default 保留厂商默认，不表示关闭推理。
use super::llm::LlmError;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReasoningEffort {
    #[default]
    Default,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    Ultra,
}

impl ReasoningEffort {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
            Self::Ultra => "ultra",
        }
    }
}

impl FromStr for ReasoningEffort {
    type Err = LlmError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "default" => Ok(Self::Default),
            "minimal" => Ok(Self::Minimal),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" => Ok(Self::Xhigh),
            "max" => Ok(Self::Max),
            "ultra" => Ok(Self::Ultra),
            _ => Err(LlmError::new("LLM reasoning effort is invalid", false)),
        }
    }
}
