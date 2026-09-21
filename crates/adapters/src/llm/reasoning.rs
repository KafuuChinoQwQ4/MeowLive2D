//! 统一推理档位解析；设置预览、保存校验和原生请求使用同一结果。
mod capabilities;

use super::multi_provider::ApiFormat;
use capabilities::{Capability, Native, capability};
use meowlive_application::ports::{llm::LlmError, reasoning::ReasoningEffort};
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReasoningResolution {
    pub requested: ReasoningEffort,
    pub effective: Option<ReasoningEffort>,
    /// 有序的非 default 档位。预算策略中的档位是应用预设，并非厂商原生枚举。
    pub supported: Vec<ReasoningEffort>,
    pub strategy: String,
    pub budget_tokens: Option<u32>,
    pub note: String,
    pub error: Option<String>,
}

pub fn resolve_reasoning(
    provider: &str,
    format: ApiFormat,
    model: &str,
    requested: ReasoningEffort,
    max_tokens: u32,
) -> ReasoningResolution {
    let caps = capability(provider, format, model);
    let mut result = ReasoningResolution {
        requested,
        effective: None,
        supported: caps.as_ref().map_or_else(Vec::new, |c| c.levels.to_vec()),
        strategy: "unsupported".into(),
        budget_tokens: None,
        note: "尚未确认此厂商、协议与模型组合的推理能力；保留选择，但不发送推理覆盖参数。".into(),
        error: None,
    };
    if requested == ReasoningEffort::Default {
        result.effective = Some(ReasoningEffort::Default);
        result.strategy = "provider_default".into();
        result.note = "不发送推理覆盖参数，保留厂商默认行为；default 不表示关闭推理。".into();
        return result;
    }
    let Some(caps) = caps else {
        return result;
    };
    let index = caps
        .levels
        .iter()
        .rposition(|&level| level <= requested)
        .unwrap_or(0);
    let effective = caps.levels[index];
    result.effective = Some(effective);
    result.strategy = match caps.native {
        Native::Openai => "openai_effort",
        Native::CompatibleEffort { strategy } => strategy,
        Native::ClaudeAdaptive => "anthropic_adaptive",
        Native::ClaudeBudget { .. } => "anthropic_budget",
        Native::GeminiLevel => "gemini_level",
        Native::GeminiBudget { .. } => "gemini_budget",
        Native::Deepseek => "deepseek_effort",
    }
    .into();
    result.note = if requested == effective {
        format!("模型支持 {}；使用该档位。", effective.as_str())
    } else {
        format!(
            "模型不支持 {}，按边界夹取、内部缺档向下取 {}。",
            requested.as_str(),
            effective.as_str()
        )
    };
    if !caps.budgets.is_empty() {
        resolve_budget(&caps, index, max_tokens, &mut result);
    }
    result
}

fn resolve_budget(
    caps: &Capability,
    index: usize,
    max_tokens: u32,
    result: &mut ReasoningResolution,
) {
    let minimum = match caps.native {
        Native::ClaudeBudget { .. } => 1024,
        Native::GeminiBudget { minimum } => minimum,
        _ => return,
    };
    let required = minimum + 512;
    if max_tokens < required {
        result.error = Some(format!(
            "此模型启用推理至少需要 {minimum} 个思考 token，并预留 512 个回答 token；请将 max_tokens 提高至至少 {required}，或选择 default。当前上限不会自动增加。"
        ));
        result.note = "当前 token 上限不足，推理配置不可使用。".into();
        return;
    }
    let requested_budget = caps.budgets[index];
    let budget = requested_budget.min(max_tokens - 512);
    result.budget_tokens = Some(budget);
    result.note.push_str(&format!(" 此策略将档位映射为应用预设预算，并非厂商原生档位，也不代表跨模型等量推理；实际思考预算 {budget} token，至少预留 512 token 给回答。"));
    if budget < requested_budget {
        result.note.push_str(&format!(
            " 原预设 {requested_budget} token 已按用户总上限 {max_tokens} 缩减。"
        ));
    }
}

/// 只负责原生参数注入，不改变消息、工具续传或缓存前缀。
pub(super) fn apply_reasoning(
    body: &mut Value,
    provider: &str,
    format: ApiFormat,
    model: &str,
    requested: ReasoningEffort,
    max_tokens: u32,
) -> Result<(), LlmError> {
    let resolution = resolve_reasoning(provider, format, model, requested, max_tokens);
    if let Some(error) = resolution.error {
        return Err(LlmError::new(error, false));
    }
    let Some(caps) = capability(provider, format, model) else {
        return Ok(());
    };
    // Even default effort must use the reasoner's inclusive output cap field.
    if matches!(caps.native, Native::Openai) && format == ApiFormat::OpenaiChat {
        body.as_object_mut()
            .expect("request body is an object")
            .remove("max_tokens");
        body["max_completion_tokens"] = json!(max_tokens);
    }
    let Some(effective) = resolution
        .effective
        .filter(|v| *v != ReasoningEffort::Default)
    else {
        return Ok(());
    };
    match caps.native {
        Native::Openai | Native::CompatibleEffort { .. } => match format {
            ApiFormat::OpenaiChat => body["reasoning_effort"] = json!(effective.as_str()),
            ApiFormat::OpenaiResponses => body["reasoning"] = json!({"effort":effective.as_str()}),
            _ => unreachable!(),
        },
        Native::ClaudeAdaptive => {
            body["thinking"] = json!({"type":"adaptive"});
            body["output_config"] = json!({"effort":effective.as_str()});
        }
        Native::ClaudeBudget { effort } => {
            body["thinking"] = json!({"type":"enabled","budget_tokens":resolution.budget_tokens});
            if effort {
                body["output_config"] = json!({"effort":effective.as_str()});
            }
        }
        Native::GeminiLevel => {
            body["generationConfig"]["thinkingConfig"] = json!({"thinkingLevel":effective.as_str()})
        }
        Native::GeminiBudget { .. } => {
            body["generationConfig"]["thinkingConfig"] =
                json!({"thinkingBudget":resolution.budget_tokens})
        }
        Native::Deepseek => match format {
            ApiFormat::OpenaiChat => {
                body["thinking"] = json!({"type":"enabled"});
                body["reasoning_effort"] = json!(effective.as_str());
            }
            ApiFormat::OpenaiResponses => body["reasoning"] = json!({"effort":effective.as_str()}),
            ApiFormat::AnthropicMessages => {
                body["thinking"] = json!({"type":"enabled"});
                body["output_config"] = json!({"effort":effective.as_str()});
            }
            _ => unreachable!(),
        },
    }
    Ok(())
}
