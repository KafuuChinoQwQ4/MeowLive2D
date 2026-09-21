mod llm_support;

use axum::{Json, Router, routing::post};
use meowlive_adapters::llm::multi_provider::{ApiFormat, MultiProvider};
use meowlive_application::ports::{llm::LanguageModel, llm_runtime::ModelOptions};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

async fn capture(model: &str, max_tokens: u32, options: ModelOptions) -> Value {
    let captured = Arc::new(Mutex::new(None));
    let seen = captured.clone();
    let (url, task) =
        llm_support::server(Router::new().fallback(post(move |Json(body): Json<Value>| {
            *seen.lock().unwrap() = Some(body);
            async {
                Json(llm_support::completion(
                    json!({"reply_to":[],"text":"你好","topic":null}),
                ))
            }
        })))
        .await;
    let mut config = llm_support::config(url);
    config.model = model.into();
    config.max_tokens = max_tokens;
    let adapter = MultiProvider::new(config, ApiFormat::OpenaiChat).unwrap();
    let result = adapter.turn(llm_support::request(vec![]), options).await;
    task.abort();
    result.unwrap();
    captured.lock().unwrap().take().unwrap()
}

#[tokio::test]
async fn default_reasoner_uses_completion_cap_without_overriding_effort() {
    let body = capture("gpt-5", 256, ModelOptions::default()).await;
    assert_eq!(body["max_completion_tokens"], 256);
    assert!(body.get("max_tokens").is_none());
    assert!(body.get("reasoning_effort").is_none());
}

#[tokio::test]
async fn cloud_accepts_user_output_cap_up_to_65536() {
    let body = capture("test-model", 65536, ModelOptions::default()).await;
    assert_eq!(body["max_tokens"], 65536);
}

use meowlive_application::ports::{
    llm_runtime::{ToolDefinition, ToolExchange, ToolResult},
    reasoning::ReasoningEffort as E,
};
use std::collections::VecDeque;

fn final_response(format: ApiFormat) -> Value {
    let answer = json!({"reply_to":[],"text":"你好","topic":null}).to_string();
    match format {
        ApiFormat::OpenaiChat => {
            json!({"choices":[{"index":0,"message":{"role":"assistant","content":answer},"finish_reason":"stop"}]})
        }
        ApiFormat::OpenaiResponses => {
            json!({"status":"completed","output":[{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":answer}]}]})
        }
        ApiFormat::AnthropicMessages => {
            json!({"type":"message","role":"assistant","stop_reason":"end_turn","content":[{"type":"text","text":answer}]})
        }
        ApiFormat::GeminiGenerateContent => {
            json!({"candidates":[{"index":0,"finishReason":"STOP","content":{"role":"model","parts":[{"text":answer}]}}]})
        }
    }
}

fn tool_response(format: ApiFormat) -> Value {
    match format {
        ApiFormat::OpenaiChat => {
            json!({"choices":[{"index":0,"finish_reason":"tool_calls","message":{"role":"assistant","content":null,"reasoning_content":"preserved-reasoning","tool_calls":[{"id":"call-1","type":"function","function":{"name":"read_time","arguments":"{}"}}]}}]})
        }
        ApiFormat::OpenaiResponses => {
            json!({"status":"completed","output":[{"type":"reasoning","id":"r1","summary":[],"encrypted_content":"preserved-signature"},{"type":"function_call","id":"fc1","call_id":"call-1","name":"read_time","arguments":"{}","status":"completed"}]})
        }
        ApiFormat::AnthropicMessages => {
            json!({"type":"message","role":"assistant","stop_reason":"tool_use","content":[{"type":"thinking","thinking":"preserved-reasoning","signature":"preserved-signature"},{"type":"tool_use","id":"call-1","name":"read_time","input":{}}]})
        }
        ApiFormat::GeminiGenerateContent => {
            json!({"candidates":[{"index":0,"finishReason":"STOP","content":{"role":"model","parts":[{"functionCall":{"id":"call-1","name":"read_time","args":{}},"thoughtSignature":"preserved-signature"}]}}]})
        }
    }
}

fn final_stream(format: ApiFormat) -> String {
    let answer = json!({"reply_to":[],"text":"你好","topic":null}).to_string();
    let frames = match format {
        ApiFormat::OpenaiChat => vec![
            json!({"choices":[{"index":0,"delta":{"role":"assistant","content":answer},"finish_reason":"stop"}]}),
        ],
        ApiFormat::OpenaiResponses => {
            vec![json!({"type":"response.completed","response":final_response(format)})]
        }
        ApiFormat::AnthropicMessages => vec![
            json!({"type":"message_start","message":{"type":"message","role":"assistant","content":[]}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":answer}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"}}),
            json!({"type":"message_stop"}),
        ],
        ApiFormat::GeminiGenerateContent => vec![final_response(format)],
    };
    let mut stream = frames
        .into_iter()
        .map(|frame| format!("data: {frame}\n\n"))
        .collect::<String>();
    if format == ApiFormat::OpenaiChat {
        stream.push_str("data: [DONE]\n\n");
    }
    stream
}

async fn native_fixture(
    format: ApiFormat,
    model: &str,
    max_tokens: u32,
    responses: Vec<String>,
    stream: bool,
) -> (
    MultiProvider,
    Arc<Mutex<Vec<Value>>>,
    tokio::task::JoinHandle<()>,
) {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let seen = captured.clone();
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    let (url, task) =
        llm_support::server(Router::new().fallback(post(move |Json(body): Json<Value>| {
            seen.lock().unwrap().push(body);
            let data = responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected HTTP call");
            async move {
                axum::response::Response::builder()
                    .header(
                        "content-type",
                        if stream {
                            "text/event-stream"
                        } else {
                            "application/json"
                        },
                    )
                    .body(axum::body::Body::from(data))
                    .unwrap()
            }
        })))
        .await;
    let mut config = llm_support::config(url);
    config.model = model.into();
    config.max_tokens = max_tokens;
    (MultiProvider::new(config, format).unwrap(), captured, task)
}

fn native_cases() -> [(
    ApiFormat,
    &'static str,
    &'static str,
    E,
    &'static str,
    Value,
); 13] {
    [
        (
            ApiFormat::OpenaiChat,
            "grok",
            "grok-4.7",
            E::Ultra,
            "/reasoning_effort",
            json!("xhigh"),
        ),
        (
            ApiFormat::OpenaiResponses,
            "grok",
            "grok-4.5",
            E::Ultra,
            "/reasoning/effort",
            json!("high"),
        ),
        (
            ApiFormat::OpenaiChat,
            "kimi",
            "kimi-k3",
            E::Medium,
            "/reasoning_effort",
            json!("low"),
        ),
        (
            ApiFormat::AnthropicMessages,
            "deepseek",
            "deepseek-v4-pro",
            E::Ultra,
            "/output_config/effort",
            json!("max"),
        ),
        (
            ApiFormat::OpenaiChat,
            "openai",
            "gpt-5",
            E::Ultra,
            "/reasoning_effort",
            json!("high"),
        ),
        (
            ApiFormat::OpenaiResponses,
            "openai",
            "gpt-6-astra",
            E::Ultra,
            "/reasoning/effort",
            json!("max"),
        ),
        (
            ApiFormat::AnthropicMessages,
            "claude",
            "claude-opus-4-6",
            E::Xhigh,
            "/output_config/effort",
            json!("high"),
        ),
        (
            ApiFormat::AnthropicMessages,
            "claude",
            "claude-sonnet-4-5",
            E::High,
            "/thinking/budget_tokens",
            json!(3584),
        ),
        (
            ApiFormat::AnthropicMessages,
            "claude",
            "claude-opus-4-5",
            E::Ultra,
            "/output_config/effort",
            json!("high"),
        ),
        (
            ApiFormat::GeminiGenerateContent,
            "gemini",
            "gemini-3-pro-preview",
            E::Medium,
            "/generationConfig/thinkingConfig/thinkingLevel",
            json!("low"),
        ),
        (
            ApiFormat::GeminiGenerateContent,
            "gemini",
            "gemini-2.5-pro",
            E::Ultra,
            "/generationConfig/thinkingConfig/thinkingBudget",
            json!(3584),
        ),
        (
            ApiFormat::OpenaiChat,
            "deepseek",
            "deepseek-flash",
            E::Medium,
            "/reasoning_effort",
            json!("low"),
        ),
        (
            ApiFormat::OpenaiResponses,
            "deepseek",
            "deepseek-v4-pro",
            E::Ultra,
            "/reasoning/effort",
            json!("max"),
        ),
    ]
}

#[tokio::test]
async fn native_parameters_survive_tool_rounds_and_preserve_signed_continuations() {
    for (format, provider, model, effort, pointer, expected) in native_cases() {
        let (adapter, captured, task) = native_fixture(
            format,
            model,
            4096,
            vec![
                tool_response(format).to_string(),
                final_response(format).to_string(),
            ],
            false,
        )
        .await;
        let mut options = ModelOptions {
            reasoning_provider: provider.into(),
            reasoning_effort: effort,
            cache_enabled: true,
            tools: vec![ToolDefinition {
                name: "read_time".into(),
                description: "Read time".into(),
                parameters_json: json!({"type":"object","properties":{}}).to_string(),
            }],
            ..Default::default()
        };
        let first = adapter
            .turn(llm_support::request(vec![]), options.clone())
            .await
            .unwrap();
        assert_eq!(first.tool_calls[0].name, "read_time");
        let continuation = first.continuation.unwrap();
        options.exchanges.push(ToolExchange {
            continuation: continuation.clone(),
            results: vec![ToolResult {
                call_id: "call-1".into(),
                name: "read_time".into(),
                content: "12:00".into(),
                is_error: false,
            }],
        });
        options.tool_choice_none = true;
        let final_turn = adapter
            .turn(llm_support::request(vec![]), options)
            .await
            .unwrap();
        task.abort();
        assert!(final_turn.decision.is_some(), "{model}");
        let bodies = captured.lock().unwrap();
        assert_eq!(bodies.len(), 2);
        for body in bodies.iter() {
            assert_eq!(body.pointer(pointer), Some(&expected), "{model}");
            match format {
                ApiFormat::OpenaiChat if provider == "openai" => {
                    assert_eq!(body["max_completion_tokens"], 4096);
                    assert!(body.get("max_tokens").is_none());
                }
                ApiFormat::OpenaiChat => {
                    assert_eq!(body["max_tokens"], 4096);
                    if provider == "deepseek" {
                        assert_eq!(body["thinking"]["type"], "enabled");
                    } else {
                        assert!(body.get("thinking").is_none());
                    }
                }
                ApiFormat::OpenaiResponses => assert_eq!(body["max_output_tokens"], 4096),
                ApiFormat::AnthropicMessages => {
                    assert_eq!(body["max_tokens"], 4096);
                    assert!(body["system"][0]["cache_control"].is_object());
                }
                ApiFormat::GeminiGenerateContent => {
                    assert_eq!(body["generationConfig"]["maxOutputTokens"], 4096)
                }
            }
        }
        let second = bodies[1].to_string();
        assert!(second.contains(if format == ApiFormat::OpenaiChat {
            "preserved-reasoning"
        } else {
            "preserved-signature"
        }));
        assert!(second.contains("12:00"));
        if format == ApiFormat::AnthropicMessages {
            assert_eq!(
                bodies[0]["thinking"]["type"],
                if model == "claude-opus-4-6" {
                    "adaptive"
                } else {
                    "enabled"
                }
            );
            assert_eq!(bodies[0]["system"], bodies[1]["system"]);
        }
    }
}

#[tokio::test]
async fn all_protocols_stream_with_the_same_reasoning_parameters() {
    for (format, provider, model, effort, pointer, expected) in native_cases() {
        let (adapter, captured, task) =
            native_fixture(format, model, 4096, vec![final_stream(format)], true).await;
        let result = adapter
            .turn(
                llm_support::request(vec![]),
                ModelOptions {
                    reasoning_provider: provider.into(),
                    reasoning_effort: effort,
                    stream: true,
                    ..Default::default()
                },
            )
            .await;
        task.abort();
        assert!(result.unwrap().decision.is_some(), "{model}");
        assert_eq!(
            captured.lock().unwrap()[0].pointer(pointer),
            Some(&expected),
            "{model}"
        );
    }
}

#[tokio::test]
async fn default_and_unknown_combinations_omit_all_reasoning_overrides() {
    for (format, provider, model, ..) in native_cases() {
        for (selected_provider, effort) in
            [(provider, E::Default), ("unverified-provider", E::Ultra)]
        {
            let (adapter, captured, task) = native_fixture(
                format,
                model,
                256,
                vec![final_response(format).to_string()],
                false,
            )
            .await;
            adapter
                .turn(
                    llm_support::request(vec![]),
                    ModelOptions {
                        reasoning_provider: selected_provider.into(),
                        reasoning_effort: effort,
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
            task.abort();
            let bodies = captured.lock().unwrap();
            for path in [
                "/reasoning",
                "/reasoning_effort",
                "/thinking",
                "/output_config/effort",
                "/generationConfig/thinkingConfig",
            ] {
                assert!(bodies[0].pointer(path).is_none(), "{model} {path}");
            }
        }
    }
}

#[tokio::test]
async fn insufficient_thinking_budget_is_rejected_before_network_io() {
    let (adapter, captured, task) = native_fixture(
        ApiFormat::AnthropicMessages,
        "claude-sonnet-4-5",
        1024,
        vec![final_response(ApiFormat::AnthropicMessages).to_string()],
        false,
    )
    .await;
    let result = adapter
        .turn(
            llm_support::request(vec![]),
            ModelOptions {
                reasoning_provider: "claude".into(),
                reasoning_effort: E::High,
                ..Default::default()
            },
        )
        .await;
    task.abort();
    assert!(result.unwrap_err().to_string().contains("1536"));
    assert!(captured.lock().unwrap().is_empty());
}
