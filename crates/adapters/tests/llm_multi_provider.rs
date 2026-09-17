mod llm_support;

use std::{
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use meowlive_adapters::llm::multi_provider::{ApiFormat, MultiProvider};
use meowlive_application::ports::llm::LanguageModel;
use serde_json::{Value, json};

#[test]
fn parses_only_supported_api_format_names() {
    assert_eq!(
        ApiFormat::from_str("openai_chat").unwrap(),
        ApiFormat::OpenaiChat
    );
    assert_eq!(
        ApiFormat::from_str("openai_responses").unwrap(),
        ApiFormat::OpenaiResponses
    );
    assert_eq!(
        ApiFormat::from_str("anthropic_messages").unwrap(),
        ApiFormat::AnthropicMessages
    );
    assert_eq!(
        ApiFormat::from_str("gemini_generate_content").unwrap(),
        ApiFormat::GeminiGenerateContent
    );
    assert!(ApiFormat::from_str("OpenAI_chat").is_err());
    assert!(ApiFormat::from_str("unknown").is_err());
}

#[tokio::test]
async fn openai_chat_format_reuses_existing_adapter_and_preserves_full_endpoint() {
    let (url, task) = llm_support::server(Router::new().route(
        "/custom/chat/completions",
        post(|headers: HeaderMap, Json(body): Json<Value>| async move {
            assert_eq!(headers["authorization"], "Bearer test-token");
            assert_eq!(body["messages"][0]["role"], "system");
            Json(llm_support::completion(json!({
                "reply_to": [], "text": "主动问候", "topic": null
            })))
        }),
    ))
    .await;
    let adapter = MultiProvider::new(
        llm_support::config(format!("{url}/custom/chat/completions")),
        ApiFormat::OpenaiChat,
    )
    .unwrap();
    let decision = adapter
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap();
    assert_eq!(decision.text.as_deref(), Some("主动问候"));
    task.abort();
}

#[tokio::test]
async fn sends_openai_responses_native_request_and_accepts_reasoning_output() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1/responses",
        post(|headers: HeaderMap, Json(body): Json<Value>| async move {
            assert_eq!(headers["authorization"], "Bearer test-token");
            assert_eq!(body["model"], "test-model");
            assert_eq!(body["store"], false);
            assert_eq!(body["max_output_tokens"], 256);
            assert_eq!(body["text"]["format"]["type"], "json_object");
            assert_eq!(body["input"][0]["role"], "system");
            assert_eq!(body["input"][0]["content"][0]["type"], "input_text");
            assert!(
                body["input"][0]["content"][0]["text"]
                    .as_str()
                    .unwrap()
                    .contains("可靠而友好")
            );
            assert_eq!(body["input"][1]["role"], "user");
            let user: Value =
                serde_json::from_str(body["input"][1]["content"][0]["text"].as_str().unwrap())
                    .unwrap();
            assert_eq!(user["events"][0]["id"], "chat-1");
            Json(json!({
                "id": "resp-test",
                "status": "completed",
                "output": [
                    {"type": "reasoning", "id": "reasoning-test", "summary": []},
                    {
                        "type": "message",
                        "id": "message-test",
                        "status": "completed",
                        "role": "assistant",
                        "content": [{
                            "type": "output_text",
                            "text": json!({
                                "reply_to": ["chat-1"], "text": "你好呀", "topic": null
                            }).to_string(),
                            "annotations": []
                        }]
                    }
                ]
            }))
        }),
    ))
    .await;
    let adapter = MultiProvider::new(
        llm_support::config(format!("{url}/v1")),
        ApiFormat::OpenaiResponses,
    )
    .unwrap();
    let decision = adapter
        .decide(llm_support::request(vec![llm_support::chat(
            "chat-1", "你好",
        )]))
        .await
        .unwrap();
    assert_eq!(decision.reply_to, ["chat-1"]);
    assert_eq!(decision.text.as_deref(), Some("你好呀"));
    task.abort();
}

#[tokio::test]
async fn sends_anthropic_messages_native_request_and_accepts_thinking_block() {
    let (url, task) = llm_support::server(Router::new().route(
        "/custom/messages",
        post(|headers: HeaderMap, Json(body): Json<Value>| async move {
            assert_eq!(headers["x-api-key"], "test-token");
            assert_eq!(headers["anthropic-version"], "2023-06-01");
            assert!(!headers.contains_key("authorization"));
            assert_eq!(body["model"], "test-model");
            assert_eq!(body["max_tokens"], 256);
            assert_eq!(body["stream"], false);
            assert!(body["system"].as_str().unwrap().contains("可靠而友好"));
            assert_eq!(body["messages"][0]["role"], "user");
            assert!(body.get("response_format").is_none());
            Json(json!({
                "id": "msg-test",
                "type": "message",
                "role": "assistant",
                "stop_reason": "end_turn",
                "content": [
                    {"type": "thinking", "thinking": "private", "signature": "sig"},
                    {"type": "text", "text": json!({
                        "reply_to": [], "text": "主动问候", "topic": null
                    }).to_string()}
                ]
            }))
        }),
    ))
    .await;
    let adapter = MultiProvider::new(
        llm_support::config(format!("{url}/custom/messages")),
        ApiFormat::AnthropicMessages,
    )
    .unwrap();
    let decision = adapter
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap();
    assert_eq!(decision.text.as_deref(), Some("主动问候"));
    task.abort();
}

#[tokio::test]
async fn anthropic_version_header_is_present_without_optional_authentication() {
    let (url, task) = llm_support::server(Router::new().route(
        "/messages",
        post(|headers: HeaderMap| async move {
            assert_eq!(headers["anthropic-version"], "2023-06-01");
            assert!(!headers.contains_key("x-api-key"));
            Json(json!({
                "type": "message",
                "role": "assistant",
                "stop_reason": "end_turn",
                "content": [{"type": "text", "text": json!({
                    "reply_to": [], "text": "主动问候", "topic": null
                }).to_string()}]
            }))
        }),
    ))
    .await;
    let mut config = llm_support::config(url);
    config.api_key = None;
    let decision = MultiProvider::new(config, ApiFormat::AnthropicMessages)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap();
    assert_eq!(decision.text.as_deref(), Some("主动问候"));
    task.abort();
}

#[tokio::test]
async fn sends_gemini_native_request_and_accepts_thought_part() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1beta/models/test-model:generateContent",
        post(|headers: HeaderMap, Json(body): Json<Value>| async move {
            assert_eq!(headers["x-goog-api-key"], "test-token");
            assert!(!headers.contains_key("authorization"));
            assert!(
                body["systemInstruction"]["parts"][0]["text"]
                    .as_str()
                    .unwrap()
                    .contains("可靠而友好")
            );
            assert_eq!(body["contents"][0]["role"], "user");
            assert_eq!(body["generationConfig"]["maxOutputTokens"], 256);
            assert_eq!(
                body["generationConfig"]["responseMimeType"],
                "application/json"
            );
            Json(json!({
                "candidates": [{
                    "finishReason": "STOP",
                    "content": {
                        "role": "model",
                        "parts": [
                            {"thought": true, "text": "private"},
                            {
                                "text": json!({
                                    "reply_to": [], "text": "主动问候", "topic": "新话题"
                                }).to_string(),
                                "thought": false,
                                "thoughtSignature": "opaque-signature"
                            }
                        ]
                    }
                }]
            }))
        }),
    ))
    .await;
    let adapter = MultiProvider::new(
        llm_support::config(format!("{url}/v1beta/models/test-model:generateContent")),
        ApiFormat::GeminiGenerateContent,
    )
    .unwrap();
    let decision = adapter
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap();
    assert_eq!(decision.text.as_deref(), Some("主动问候"));
    assert_eq!(decision.topic.as_deref(), Some("新话题"));
    task.abort();
}

#[tokio::test]
async fn gemini_full_endpoint_replaces_the_model_segment_from_config() {
    let old_endpoint_contacted = Arc::new(AtomicBool::new(false));
    let marker = old_endpoint_contacted.clone();
    let (url, task) = llm_support::server(
        Router::new()
            .route(
                "/v1beta/models/old-model:generateContent",
                post(move || async move {
                    marker.store(true, Ordering::SeqCst);
                    StatusCode::INTERNAL_SERVER_ERROR
                }),
            )
            .route(
                "/v1beta/models/new-model:generateContent",
                post(|| async {
                    Json(json!({
                        "candidates": [{
                            "finishReason": "STOP",
                            "content": {"role": "model", "parts": [{"text": json!({
                                "reply_to": [], "text": "新模型", "topic": null
                            }).to_string()}]}
                        }]
                    }))
                }),
            ),
    )
    .await;
    let mut config = llm_support::config(format!("{url}/v1beta/models/old-model:generateContent/"));
    config.model = "new-model".into();
    let decision = MultiProvider::new(config, ApiFormat::GeminiGenerateContent)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap();
    assert_eq!(decision.text.as_deref(), Some("新模型"));
    assert!(!old_endpoint_contacted.load(Ordering::SeqCst));
    task.abort();
}

#[tokio::test]
async fn rejects_incomplete_and_tool_outputs_for_native_protocols() {
    let cases = [
        (
            ApiFormat::OpenaiResponses,
            "/responses",
            json!({"status": "incomplete", "output": []}),
        ),
        (
            ApiFormat::OpenaiResponses,
            "/responses",
            json!({"status": "completed", "output": [{
                "type": "function_call", "name": "shell", "arguments": "{}"
            }]}),
        ),
        (
            ApiFormat::AnthropicMessages,
            "/messages",
            json!({
                "type": "message", "role": "assistant", "stop_reason": "max_tokens",
                "content": [{"type": "text", "text": "{}"}]
            }),
        ),
        (
            ApiFormat::AnthropicMessages,
            "/messages",
            json!({
                "type": "message", "role": "assistant", "stop_reason": "end_turn",
                "content": [{"type": "tool_use", "name": "shell", "input": {}}]
            }),
        ),
        (
            ApiFormat::GeminiGenerateContent,
            "/models/test-model:generateContent",
            json!({"candidates": [{
                "finishReason": "MAX_TOKENS", "content": {"role": "model", "parts": []}
            }]}),
        ),
        (
            ApiFormat::GeminiGenerateContent,
            "/models/test-model:generateContent",
            json!({"candidates": [{
                "finishReason": "STOP", "content": {"role": "model", "parts": [{
                    "functionCall": {"name": "shell", "args": {}}
                }]}
            }]}),
        ),
        (
            ApiFormat::GeminiGenerateContent,
            "/models/test-model:generateContent",
            json!({"candidates": [{
                "finishReason": "STOP", "content": {"role": "model", "parts": [{
                    "text": "{}", "thought": "false"
                }]}
            }]}),
        ),
    ];

    for (format, path, response) in cases {
        let router = Router::new().route(path, post(move || async move { Json(response) }));
        let (url, task) = llm_support::server(router).await;
        let error = MultiProvider::new(llm_support::config(url), format)
            .unwrap()
            .decide(llm_support::request(Vec::new()))
            .await
            .unwrap_err();
        assert!(!error.retryable);
        assert!(error.message.contains("invalid"));
        task.abort();
    }
}

#[tokio::test]
async fn multi_provider_transport_is_bounded_and_does_not_follow_redirects() {
    let contacted = Arc::new(AtomicBool::new(false));
    let marker = contacted.clone();
    let (url, task) = llm_support::server(
        Router::new()
            .route(
                "/responses",
                post(|| async { (StatusCode::TEMPORARY_REDIRECT, [("location", "/private")]) }),
            )
            .route(
                "/private",
                post(move || async move {
                    marker.store(true, Ordering::SeqCst);
                    Json(json!({"status": "completed", "output": []}))
                }),
            ),
    )
    .await;
    let error = MultiProvider::new(llm_support::config(url), ApiFormat::OpenaiResponses)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap_err();
    assert!(!error.retryable);
    assert!(error.message.contains("307"));
    assert!(!contacted.load(Ordering::SeqCst));
    task.abort();

    let (url, task) =
        llm_support::server(Router::new().route("/messages", post(|| async { vec![b'x'; 2048] })))
            .await;
    let mut config = llm_support::config(url);
    config.max_response_bytes = 1024;
    let error = MultiProvider::new(config, ApiFormat::AnthropicMessages)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap_err();
    assert!(error.message.contains("limit"));
    assert!(!error.message.contains("test-token"));
    task.abort();
}

#[tokio::test]
async fn multi_provider_timeout_covers_response_body_and_hides_upstream_details() {
    use tokio::io::AsyncWriteExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        crate::raw_http::consume_request(&mut stream).await;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nx\r\n")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
    });
    let mut config = llm_support::config(url);
    config.timeout = Duration::from_secs(1);
    let error = MultiProvider::new(config, ApiFormat::GeminiGenerateContent)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap_err();
    assert!(error.retryable);
    assert!(error.message.contains("timed out"));
    assert!(!error.message.contains("test-token"));
    task.abort();

    let (url, task) = llm_support::server(Router::new().route(
        "/messages",
        post(|| async { (StatusCode::BAD_REQUEST, "private upstream body test-token") }),
    ))
    .await;
    let error = MultiProvider::new(llm_support::config(url), ApiFormat::AnthropicMessages)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap_err();
    assert!(error.message.contains("400"));
    assert!(!error.message.contains("private upstream body"));
    assert!(!error.message.contains("test-token"));
    task.abort();
}

#[path = "support/raw_http.rs"]
mod raw_http;
