//! 2026-09-22 核对的具体模型能力。显式厂商与协议优先；custom 仅按已知 ID 和用户选择的协议推断。
//! OpenAI: https://developers.openai.com/api/docs/models/{model}
//! Claude: https://platform.claude.com/docs/en/build-with-claude/effort
//!         https://platform.claude.com/docs/en/build-with-claude/extended-thinking
//! Gemini: https://ai.google.dev/gemini-api/docs/generate-content/thinking
//! DeepSeek: https://api-docs.deepseek.com/guides/thinking_mode/
//!           https://api-docs.deepseek.com/quick_start/pricing/
//! Grok: https://docs.x.ai/developers/model-capabilities/text/reasoning
//! Kimi: https://platform.kimi.ai/docs/guide/use-reasoning-effort
use super::{ApiFormat, ReasoningEffort};
use ReasoningEffort::{High, Low, Max, Medium, Minimal, Ultra, Xhigh};

#[derive(Clone, Copy)]
pub(super) enum Native {
    Openai,
    CompatibleEffort { strategy: &'static str },
    ClaudeAdaptive,
    ClaudeBudget { effort: bool },
    GeminiLevel,
    GeminiBudget { minimum: u32 },
    Deepseek,
}

pub(super) struct Capability {
    pub native: Native,
    pub levels: &'static [ReasoningEffort],
    pub budgets: &'static [u32],
}

const THREE: &[ReasoningEffort] = &[Low, Medium, High];
const FOUR: &[ReasoningEffort] = &[Minimal, Low, Medium, High];
const XHIGH: &[ReasoningEffort] = &[Low, Medium, High, Xhigh];
const MAX: &[ReasoningEffort] = &[Low, Medium, High, Xhigh, Max];
const ALL: &[ReasoningEffort] = &[Minimal, Low, Medium, High, Xhigh, Max, Ultra];

pub(super) fn capability(provider: &str, format: ApiFormat, model: &str) -> Option<Capability> {
    let provider = provider.trim();
    let model = model.trim();
    let inferred = provider.is_empty() || provider == "custom";
    if (inferred || provider == "openai")
        && matches!(format, ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses)
    {
        let levels = match model {
            "gpt-5.2-pro" | "gpt-5.2-pro-2025-12-11" | "gpt-5.4-pro" | "gpt-5.4-pro-2026-03-05"
                if format == ApiFormat::OpenaiResponses =>
            {
                Some(&[Medium, High, Xhigh][..])
            }
            "gpt-5"
            | "gpt-5-2025-08-07"
            | "gpt-5-mini"
            | "gpt-5-mini-2025-08-07"
            | "gpt-5-nano"
            | "gpt-5-nano-2025-08-07" => Some(FOUR),
            "gpt-5.1" | "gpt-5.1-2025-11-13" | "o3" | "o3-2025-04-16" | "o4-mini"
            | "o4-mini-2025-04-16" | "o1" | "o1-2024-12-17" => Some(THREE),
            "gpt-5.2" | "gpt-5.2-2025-12-11" | "gpt-5.4" | "gpt-5.4-2026-03-05" | "gpt-5.5" => {
                Some(XHIGH)
            }
            "gpt-5.6" | "gpt-5.6-sol" | "gpt-5.6-terra" | "gpt-5.6-luna" | "gpt-6-astra" => {
                Some(MAX)
            }
            _ => None,
        };
        if let Some(levels) = levels {
            return Some(Capability {
                native: Native::Openai,
                levels,
                budgets: &[],
            });
        }
    }
    if (inferred || matches!(provider, "grok" | "xai"))
        && matches!(format, ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses)
    {
        let levels = match model {
            "grok-4.5" => Some(THREE),
            "grok-4.6" | "grok-4.7" => Some(XHIGH),
            _ => None,
        };
        if let Some(levels) = levels {
            return Some(Capability {
                native: Native::CompatibleEffort {
                    strategy: "grok_effort",
                },
                levels,
                budgets: &[],
            });
        }
    }
    // K3's verified Chat API uses effort only, unlike K2.x's thinking switch.
    if (inferred || provider == "kimi") && format == ApiFormat::OpenaiChat && model == "kimi-k3" {
        return Some(Capability {
            native: Native::CompatibleEffort {
                strategy: "kimi_effort",
            },
            levels: &[Low, High, Max],
            budgets: &[],
        });
    }
    if (inferred || matches!(provider, "anthropic" | "claude"))
        && format == ApiFormat::AnthropicMessages
    {
        let levels = match model {
            "claude-opus-4-6" | "claude-sonnet-4-6" | "claude-mythos-preview" => {
                Some(&[Low, Medium, High, Max][..])
            }
            "claude-opus-4-7" | "claude-opus-4-8" | "claude-opus-5" | "claude-sonnet-5"
            | "claude-fable-5" | "claude-fable-5-1" | "claude-mythos-5" | "claude-mythos-5-1" => {
                Some(MAX)
            }
            _ => None,
        };
        if let Some(levels) = levels {
            return Some(Capability {
                native: Native::ClaudeAdaptive,
                levels,
                budgets: &[],
            });
        }
        if matches!(model, "claude-opus-4-5" | "claude-opus-4-5-20251101") {
            return Some(Capability {
                native: Native::ClaudeBudget { effort: true },
                levels: THREE,
                budgets: &[1024, 4096, 16384],
            });
        }
        if matches!(
            model,
            "claude-sonnet-4-5"
                | "claude-sonnet-4-5-20250929"
                | "claude-haiku-4-5"
                | "claude-haiku-4-5-20251001"
                | "claude-sonnet-4-0"
                | "claude-sonnet-4-20250514"
                | "claude-opus-4-0"
                | "claude-opus-4-20250514"
                | "claude-opus-4-1"
                | "claude-opus-4-1-20250805"
        ) {
            return Some(Capability {
                native: Native::ClaudeBudget { effort: false },
                levels: ALL,
                budgets: &[1024, 2048, 4096, 8192, 16384, 24576, 32768],
            });
        }
    }
    if (inferred || provider == "gemini") && format == ApiFormat::GeminiGenerateContent {
        let model = model.strip_prefix("models/").unwrap_or(model);
        let levels = match model {
            "gemini-3-pro-preview" => Some(&[Low, High][..]),
            "gemini-3.1-pro-preview" | "gemini-3.7-flash" | "gemini-3.8-flash" => Some(THREE),
            "gemini-3-flash-preview"
            | "gemini-3.1-flash-lite-preview"
            | "gemini-3.1-flash-lite"
            | "gemini-3.5-flash-lite"
            | "gemini-3.5-flash"
            | "gemini-3.6-flash" => Some(FOUR),
            _ => None,
        };
        if let Some(levels) = levels {
            return Some(Capability {
                native: Native::GeminiLevel,
                levels,
                budgets: &[],
            });
        }
        let (budgets, minimum): (&[u32], u32) = match model {
            "gemini-2.5-pro" => (&[128, 1024, 4096, 8192, 16384, 24576, 32768], 128),
            // Budget 0 disables thinking; explicit nondefault efforts retain at least one token.
            "gemini-2.5-flash" => (&[128, 512, 1024, 4096, 8192, 16384, 24576], 1),
            "gemini-2.5-flash-lite" => (&[512, 1024, 2048, 4096, 8192, 16384, 24576], 512),
            _ => (&[], 0),
        };
        if !budgets.is_empty() {
            return Some(Capability {
                native: Native::GeminiBudget { minimum },
                levels: ALL,
                budgets,
            });
        }
    }
    if (inferred || provider == "deepseek")
        && matches!(
            format,
            ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses | ApiFormat::AnthropicMessages
        )
        && matches!(
            model,
            "deepseek-flash"
                | "deepseek-v4-pro"
                | "deepseek-v4-flash"
                | "deepseek-v4-flash-vision-exp"
        )
    {
        return Some(Capability {
            native: Native::Deepseek,
            levels: &[Low, High, Max],
            budgets: &[],
        });
    }
    None
}
