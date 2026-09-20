#![allow(dead_code)]

use axum::Router;
use meowlive_adapters::llm::openai_compatible::LlmConfig;
use meowlive_application::ports::llm::{ConversationTurn, DecisionRequest};
use meowlive_domain::event::{EventKind, LiveEvent};
use serde_json::json;
use std::time::Duration;

pub async fn server(router: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (url, task)
}

pub fn config(base_url: String) -> LlmConfig {
    LlmConfig {
        base_url,
        model: "test-model".into(),
        api_key: Some("test-token".into()),
        timeout: Duration::from_secs(2),
        max_response_bytes: 16 * 1024,
        max_tokens: 256,
        json_mode: true,
    }
}

pub fn request(events: Vec<LiveEvent>) -> DecisionRequest {
    DecisionRequest {
        memory_context: vec![],
        persona: "你是可靠而友好的猫娘主播。".into(),
        topic: "测试直播".into(),
        events,
        history: vec![ConversationTurn {
            user: "观众甲说：早上好".into(),
            assistant: "早上好，欢迎回来。".into(),
        }],
    }
}

pub fn chat(id: &str, text: &str) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "simulator".into(),
        viewer: "观众甲".into(),
        viewer_identity: None,
        occurred_at_ms: 1234,
        gift_metadata: None,
        kind: EventKind::Chat { text: text.into() },
    }
}

pub fn gift(id: &str, count: u32) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "simulator".into(),
        viewer: "观众甲".into(),
        viewer_identity: None,
        occurred_at_ms: 1234,
        gift_metadata: None,
        kind: EventKind::Gift {
            name: "小鱼干".into(),
            count,
        },
    }
}

pub fn completion(content: serde_json::Value) -> serde_json::Value {
    json!({
        "id": "completion-test",
        "choices": [{
            "message": {"role": "assistant", "content": content.to_string()},
            "finish_reason": "stop"
        }]
    })
}
