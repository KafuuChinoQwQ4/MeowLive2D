use meowlive_adapters::llm::{multi_provider::ApiFormat as F, reasoning::resolve_reasoning};
use meowlive_application::ports::reasoning::ReasoningEffort as E;

#[test]
fn levels_parse_exactly_and_default_is_an_omission() {
    for value in [
        "default", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
    ] {
        assert_eq!(value.parse::<E>().unwrap().as_str(), value);
    }
    for invalid in ["none", "LOW", "", " high", "adaptive"] {
        assert!(invalid.parse::<E>().is_err());
    }
    let resolved = resolve_reasoning(
        "anthropic",
        F::AnthropicMessages,
        "claude-sonnet-4-5",
        E::default(),
        64,
    );
    assert_eq!(resolved.effective, Some(E::Default));
    assert_eq!(resolved.budget_tokens, None);
    assert!(resolved.error.is_none());
}

#[test]
fn native_levels_match_clamp_and_floor_gaps() {
    for (provider, format, model, requested, expected) in [
        ("openai", F::OpenaiChat, "gpt-5", E::Minimal, E::Minimal),
        ("openai", F::OpenaiChat, "gpt-5", E::Ultra, E::High),
        ("openai", F::OpenaiResponses, "gpt-5.4", E::Minimal, E::Low),
        ("openai", F::OpenaiResponses, "gpt-5.4", E::Ultra, E::Xhigh),
        (
            "openai",
            F::OpenaiResponses,
            "gpt-6-astra",
            E::Ultra,
            E::Max,
        ),
        (
            "anthropic",
            F::AnthropicMessages,
            "claude-opus-4-6",
            E::Xhigh,
            E::High,
        ),
        (
            "anthropic",
            F::AnthropicMessages,
            "claude-opus-4-6",
            E::Max,
            E::Max,
        ),
        (
            "anthropic",
            F::AnthropicMessages,
            "claude-sonnet-4-6",
            E::Ultra,
            E::Max,
        ),
        (
            "anthropic",
            F::AnthropicMessages,
            "claude-opus-4-7",
            E::Xhigh,
            E::Xhigh,
        ),
        (
            "gemini",
            F::GeminiGenerateContent,
            "gemini-3-pro-preview",
            E::Medium,
            E::Low,
        ),
        (
            "gemini",
            F::GeminiGenerateContent,
            "gemini-3-flash-preview",
            E::Minimal,
            E::Minimal,
        ),
        (
            "gemini",
            F::GeminiGenerateContent,
            "gemini-3.1-pro-preview",
            E::Medium,
            E::Medium,
        ),
        (
            "deepseek",
            F::OpenaiChat,
            "deepseek-flash",
            E::Medium,
            E::Low,
        ),
        (
            "deepseek",
            F::OpenaiChat,
            "deepseek-v4-pro",
            E::Ultra,
            E::Max,
        ),
    ] {
        let result = resolve_reasoning(provider, format, model, requested, 16384);
        assert_eq!(result.effective, Some(expected), "{model}: {requested:?}");
        assert!(!result.supported.contains(&E::Default));
        assert!(result.supported.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(result.error.is_none());
    }
}

#[test]
fn unknown_provider_model_or_wrong_protocol_never_guesses_parameters() {
    for (provider, format, model) in [
        ("randomvendor", F::OpenaiChat, "gpt-5"),
        ("local", F::OpenaiChat, "gpt-5"),
        ("openai", F::OpenaiChat, "gpt-5.99"),
        ("custom", F::OpenaiChat, "gpt-5-unverified-date"),
        ("openai", F::AnthropicMessages, "gpt-5"),
        ("gemini", F::OpenaiChat, "gemini-3-flash-preview"),
        ("anthropic", F::AnthropicMessages, "claude-opus-9"),
    ] {
        let result = resolve_reasoning(provider, format, model, E::Ultra, 4096);
        assert!(result.effective.is_none(), "{provider}: {model}");
        assert!(result.supported.is_empty());
        assert!(result.error.is_none());
        assert!(!result.note.is_empty());
    }
    for provider in ["", "custom"] {
        assert_eq!(
            resolve_reasoning(provider, F::OpenaiChat, "gpt-5", E::Low, 4096).effective,
            Some(E::Low)
        );
    }
}

#[test]
fn budget_models_keep_answer_headroom_and_reject_insufficient_caps() {
    for (provider, format, model, minimum) in [
        ("anthropic", F::AnthropicMessages, "claude-sonnet-4-5", 1024),
        ("gemini", F::GeminiGenerateContent, "gemini-2.5-pro", 128),
        (
            "gemini",
            F::GeminiGenerateContent,
            "gemini-2.5-flash-lite",
            512,
        ),
    ] {
        let invalid = resolve_reasoning(provider, format, model, E::Low, minimum + 511);
        assert!(invalid.error.is_some(), "{model}");
        let valid = resolve_reasoning(provider, format, model, E::Ultra, minimum + 512);
        assert_eq!(valid.budget_tokens, Some(minimum), "{model}");
        assert!(valid.error.is_none());
        assert!(valid.note.contains("512"));
    }
    let mut previous = 0;
    for effort in [
        E::Minimal,
        E::Low,
        E::Medium,
        E::High,
        E::Xhigh,
        E::Max,
        E::Ultra,
    ] {
        let resolved = resolve_reasoning(
            "gemini",
            F::GeminiGenerateContent,
            "gemini-2.5-pro",
            effort,
            65536,
        );
        let budget = resolved.budget_tokens.unwrap();
        assert!(budget >= previous && budget <= 32768);
        previous = budget;
    }
    assert_eq!(
        resolve_reasoning(
            "anthropic",
            F::AnthropicMessages,
            "claude-sonnet-4-5",
            E::High,
            4096
        )
        .budget_tokens,
        Some(3584)
    );
}

#[test]
fn preview_matches_adapter_normalization_and_claude_ui_provider_name() {
    let normalized = resolve_reasoning(
        "claude",
        F::AnthropicMessages,
        "claude-opus-4-6",
        E::Xhigh,
        4096,
    );
    assert_eq!(
        resolve_reasoning(
            " claude ",
            F::AnthropicMessages,
            " claude-opus-4-6 ",
            E::Xhigh,
            4096
        ),
        normalized
    );
    assert_eq!(
        resolve_reasoning(
            "anthropic",
            F::AnthropicMessages,
            "claude-opus-4-6",
            E::Xhigh,
            4096
        ),
        normalized
    );
}

#[test]
fn pro_models_are_responses_only_and_clamp_to_their_higher_minimum() {
    for model in ["gpt-5.2-pro", "gpt-5.4-pro"] {
        assert_eq!(
            resolve_reasoning("openai", F::OpenaiResponses, model, E::Minimal, 4096).effective,
            Some(E::Medium)
        );
        assert_eq!(
            resolve_reasoning("openai", F::OpenaiResponses, model, E::Ultra, 4096).effective,
            Some(E::Xhigh)
        );
        assert_eq!(
            resolve_reasoning("openai", F::OpenaiChat, model, E::High, 4096).effective,
            None
        );
    }
}

#[test]
fn grok_native_effort_does_not_infer_capabilities_for_other_models() {
    for (model, expected) in [
        ("grok-4.5", E::High),
        ("grok-4.6", E::Xhigh),
        ("grok-4.7", E::Xhigh),
    ] {
        for format in [F::OpenaiChat, F::OpenaiResponses] {
            assert_eq!(
                resolve_reasoning("grok", format, model, E::Ultra, 4096).effective,
                Some(expected)
            );
        }
    }
    assert_eq!(
        resolve_reasoning(
            "grok",
            F::OpenaiResponses,
            "grok-4.20-multi-agent",
            E::High,
            4096
        )
        .effective,
        None
    );
    assert_eq!(
        resolve_reasoning("grok", F::OpenaiChat, "grok-5-future", E::High, 4096).effective,
        None
    );
}

#[test]
fn kimi_k3_native_levels_floor_without_inventing_k2_levels() {
    assert_eq!(
        resolve_reasoning("kimi", F::OpenaiChat, "kimi-k3", E::Medium, 4096).effective,
        Some(E::Low)
    );
    assert_eq!(
        resolve_reasoning("kimi", F::OpenaiChat, "kimi-k3", E::Ultra, 4096).effective,
        Some(E::Max)
    );
    assert_eq!(
        resolve_reasoning("kimi", F::OpenaiChat, "kimi-k2.7-code", E::High, 4096).effective,
        None
    );
}
