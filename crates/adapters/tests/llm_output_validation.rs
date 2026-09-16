mod llm_support;

use axum::{Json, Router, routing::post};
use meowlive_adapters::llm::openai_compatible::OpenAiCompatible;
use meowlive_application::ports::llm::{AgentDecision, LanguageModel};
use serde_json::{Value, json};

async fn decide_with(
    events: Vec<meowlive_domain::event::LiveEvent>,
    response: Value,
) -> Result<AgentDecision, meowlive_application::ports::llm::LlmError> {
    let (url, task) = llm_support::server(Router::new().route(
        "/chat/completions",
        post(move || {
            let response = response.clone();
            async move { Json(response) }
        }),
    ))
    .await;
    let result = OpenAiCompatible::new(llm_support::config(url))
        .unwrap()
        .decide(llm_support::request(events))
        .await;
    task.abort();
    result
}

#[tokio::test]
async fn accepts_valid_event_reply() {
    let event = llm_support::chat("chat-1", "你好");
    let decision = decide_with(
        vec![event],
        llm_support::completion(json!({
            "reply_to": ["chat-1"], "text": "你好呀", "topic": "问候"
        })),
    )
    .await
    .unwrap();
    assert_eq!(decision.reply_to, ["chat-1"]);
    assert_eq!(decision.topic.as_deref(), Some("问候"));
}

#[tokio::test]
async fn accepts_topic_only_event_ignore() {
    let ignored = decide_with(
        vec![llm_support::chat("chat-1", "你好")],
        llm_support::completion(json!({
            "reply_to": [], "text": null, "topic": "下个话题"
        })),
    )
    .await
    .unwrap();
    assert_eq!(ignored.text, None);
    assert_eq!(ignored.topic.as_deref(), Some("下个话题"));
}

#[tokio::test]
async fn rejects_unknown_duplicate_or_missing_reply_targets() {
    for content in [
        json!({"reply_to": ["unknown"], "text": "你好", "topic": null}),
        json!({"reply_to": ["chat-1", "chat-1"], "text": "你好", "topic": null}),
        json!({"reply_to": [], "text": "你好", "topic": null}),
        json!({"reply_to": ["chat-1"], "text": null, "topic": null}),
    ] {
        let error = decide_with(
            vec![llm_support::chat("chat-1", "你好")],
            llm_support::completion(content),
        )
        .await
        .unwrap_err();
        assert!(!error.retryable);
    }
}

#[tokio::test]
async fn rejects_malformed_or_extra_decision_fields() {
    for content in [
        Value::String("private-body test-token https://secret.invalid".into()),
        json!({"reply_to": [], "text": "hi", "topic": null, "action": "shutdown"}),
        json!({"reply_to": "chat-1", "text": "hi", "topic": null}),
        json!({"reply_to": [], "text": " ", "topic": null}),
        json!({"reply_to": [], "text": "x".repeat(501), "topic": null}),
        json!({"reply_to": [], "text": "hi", "topic": "x".repeat(201)}),
    ] {
        let message = decide_with(Vec::new(), llm_support::completion(content))
            .await
            .unwrap_err()
            .message;
        assert!(message.contains("model"));
        assert!(!message.contains("private-body"));
        assert!(!message.contains("test-token"));
        assert!(!message.contains("secret.invalid"));
    }
}

#[tokio::test]
async fn rejects_tool_calls_multiple_choices_and_non_stop_finishes() {
    let valid = json!({"reply_to": [], "text": "hi", "topic": null}).to_string();
    let responses = [
        json!({"choices": [{
            "message": {"content": valid, "tool_calls": []}, "finish_reason": "stop"
        }]}),
        json!({"choices": [{
            "message": {"content": valid, "tool_calls": null}, "finish_reason": "stop"
        }]}),
        json!({"choices": [
            {"message": {"content": valid}, "finish_reason": "stop"},
            {"message": {"content": valid}, "finish_reason": "stop"}
        ]}),
        json!({"choices": [{"message": {"content": valid}, "finish_reason": "length"}]}),
        json!({"choices": [{"message": {"content": null}, "finish_reason": "stop"}]}),
        json!({"choices": []}),
    ];
    for response in responses {
        let error = decide_with(Vec::new(), response).await.unwrap_err();
        assert!(!error.retryable);
    }
}
