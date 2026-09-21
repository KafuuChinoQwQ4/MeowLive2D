use super::{ApiFormat, invalid};
use meowlive_application::ports::{
    llm::LlmError,
    llm_runtime::{TokenUsage, ToolCall, ToolDefinition},
};
use serde_json::{Value, json};
use std::collections::HashSet;

pub(super) struct Output {
    pub text: String,
    pub calls: Vec<ToolCall>,
    pub continuation: Value,
    pub tool_finish: bool,
}

pub(super) fn usage(format: ApiFormat, raw: &Value) -> TokenUsage {
    let u = &raw[if format == ApiFormat::GeminiGenerateContent {
        "usageMetadata"
    } else {
        "usage"
    }];
    match format {
        ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses => {
            let (i, o, id, od) = if format == ApiFormat::OpenaiChat {
                (
                    "prompt_tokens",
                    "completion_tokens",
                    "prompt_tokens_details",
                    "completion_tokens_details",
                )
            } else {
                (
                    "input_tokens",
                    "output_tokens",
                    "input_tokens_details",
                    "output_tokens_details",
                )
            };
            TokenUsage {
                input_tokens: u[i].as_u64(),
                output_tokens: u[o].as_u64(),
                cache_read_tokens: u[id]["cached_tokens"].as_u64().or_else(|| {
                    (format == ApiFormat::OpenaiChat)
                        .then(|| u["prompt_cache_hit_tokens"].as_u64())
                        .flatten()
                }),
                cache_write_tokens: u[id]["cache_write_tokens"].as_u64(),
                reasoning_tokens: u[od]["reasoning_tokens"].as_u64(),
            }
        }
        ApiFormat::AnthropicMessages => {
            let read = u["cache_read_input_tokens"].as_u64();
            let write = u["cache_creation_input_tokens"].as_u64();
            TokenUsage {
                input_tokens: u["input_tokens"]
                    .as_u64()
                    .and_then(|v| v.checked_add(read.unwrap_or(0)))
                    .and_then(|v| v.checked_add(write.unwrap_or(0))),
                output_tokens: u["output_tokens"].as_u64(),
                cache_read_tokens: read,
                cache_write_tokens: write,
                reasoning_tokens: u["output_tokens_details"]["thinking_tokens"].as_u64(),
            }
        }
        ApiFormat::GeminiGenerateContent => {
            let thinking = u["thoughtsTokenCount"].as_u64();
            TokenUsage {
                input_tokens: u["promptTokenCount"].as_u64(),
                output_tokens: u["candidatesTokenCount"]
                    .as_u64()
                    .and_then(|v| v.checked_add(thinking.unwrap_or(0))),
                cache_read_tokens: u["cachedContentTokenCount"].as_u64(),
                cache_write_tokens: None,
                reasoning_tokens: thinking,
            }
        }
    }
}
pub(super) fn parse(
    format: ApiFormat,
    raw: &Value,
    tools: &[ToolDefinition],
) -> Result<Output, LlmError> {
    if raw.get("error").is_some_and(|value| !value.is_null()) {
        return Err(invalid());
    }
    let mut text = String::new();
    let mut calls = Vec::new();
    let (continuation, tool_finish) = match format {
        ApiFormat::OpenaiChat => {
            let choices = array(&raw["choices"])?;
            if choices.len() != 1 {
                return Err(invalid());
            }
            let c = &choices[0];
            let message = &c["message"];
            if ["function_call", "audio"]
                .iter()
                .any(|key| message.get(key).is_some_and(|value| !value.is_null()))
            {
                return Err(invalid());
            }
            if message["role"] != "assistant"
                || message.get("refusal").is_some_and(|v| !v.is_null())
            {
                return Err(invalid());
            }
            if !message["content"].is_null() {
                text.push_str(string(&message["content"])?);
            }
            if let Some(raw_calls) = message.get("tool_calls").filter(|v| !v.is_null()) {
                for c in array(raw_calls)? {
                    if c["type"] != "function" {
                        return Err(invalid());
                    }
                    calls.push(call(
                        &c["id"],
                        &c["function"]["name"],
                        &c["function"]["arguments"],
                        true,
                    )?);
                }
            }
            let tool_finish = match c["finish_reason"].as_str() {
                Some("stop") => false,
                Some("tool_calls") => true,
                _ => return Err(invalid()),
            };
            (message.clone(), tool_finish)
        }
        ApiFormat::OpenaiResponses => {
            if raw["status"] != "completed" {
                return Err(invalid());
            }
            let items = array(&raw["output"])?;
            for item in items {
                match item["type"].as_str() {
                    Some("reasoning") => {}
                    Some("function_call") => {
                        if item.get("status").is_some_and(|v| v != "completed") {
                            return Err(invalid());
                        }
                        calls.push(call(
                            &item["call_id"],
                            &item["name"],
                            &item["arguments"],
                            true,
                        )?);
                    }
                    Some("message") => {
                        if item["role"] != "assistant" || item["status"] != "completed" {
                            return Err(invalid());
                        }
                        for part in array(&item["content"])? {
                            if part["type"] != "output_text" {
                                return Err(invalid());
                            }
                            text.push_str(string(&part["text"])?);
                        }
                    }
                    _ => return Err(invalid()),
                }
            }
            (Value::Array(items.clone()), !calls.is_empty())
        }
        ApiFormat::AnthropicMessages => {
            if raw["type"] != "message" || raw["role"] != "assistant" {
                return Err(invalid());
            }
            let parts = array(&raw["content"])?;
            for part in parts {
                match part["type"].as_str() {
                    Some("thinking") => {
                        string(&part["thinking"])?;
                        string(&part["signature"])?;
                    }
                    Some("redacted_thinking") => {
                        string(&part["data"])?;
                    }
                    Some("text") => text.push_str(string(&part["text"])?),
                    Some("tool_use") => {
                        calls.push(call(&part["id"], &part["name"], &part["input"], false)?)
                    }
                    _ => return Err(invalid()),
                }
            }
            let tool_finish = match raw["stop_reason"].as_str() {
                Some("end_turn" | "stop_sequence") => false,
                Some("tool_use") => true,
                _ => return Err(invalid()),
            };
            (json!({"role":"assistant","content":parts}), tool_finish)
        }
        ApiFormat::GeminiGenerateContent => {
            let candidates = array(&raw["candidates"])?;
            if candidates.len() != 1 || candidates[0]["finishReason"] != "STOP" {
                return Err(invalid());
            }
            let content = &candidates[0]["content"];
            if content["role"] != "model" {
                return Err(invalid());
            }
            for (index, part) in array(&content["parts"])?.iter().enumerate() {
                let object = part.as_object().ok_or_else(invalid)?;
                if object.keys().any(|key| {
                    !matches!(
                        key.as_str(),
                        "text" | "thought" | "thoughtSignature" | "functionCall"
                    )
                }) {
                    return Err(invalid());
                }
                if part.get("thought").is_some_and(|v| !v.is_boolean()) {
                    return Err(invalid());
                }
                if let Some(c) = part.get("functionCall") {
                    if part.get("text").is_some() || part["thought"] == true {
                        return Err(invalid());
                    }
                    let id = c
                        .get("id")
                        .cloned()
                        .unwrap_or_else(|| json!(format!("gemini-{index}")));
                    calls.push(call(&id, &c["name"], &c["args"], false)?);
                } else {
                    let value = string(&part["text"])?;
                    if part["thought"] != true {
                        text.push_str(value);
                    }
                }
            }
            (content.clone(), !calls.is_empty())
        }
    };
    if tool_finish == calls.is_empty() || calls.len() > 4 {
        return Err(invalid());
    }
    let mut ids = HashSet::new();
    for call in &calls {
        if !ids.insert(&call.id) || !tools.iter().any(|t| t.name == call.name) {
            return Err(invalid());
        }
    }
    if calls.is_empty() && text.trim().is_empty() {
        return Err(invalid());
    }
    Ok(Output {
        text,
        calls,
        continuation,
        tool_finish,
    })
}
pub(super) fn continuation_response(format: ApiFormat, value: Value) -> Value {
    match format {
        ApiFormat::OpenaiChat => {
            json!({"choices":[{"finish_reason":"tool_calls","message":value}]})
        }
        ApiFormat::OpenaiResponses => json!({"status":"completed","output":value}),
        ApiFormat::AnthropicMessages => {
            json!({"type":"message","role":value["role"],"content":value["content"],"stop_reason":"tool_use"})
        }
        ApiFormat::GeminiGenerateContent => {
            json!({"candidates":[{"content":value,"finishReason":"STOP"}]})
        }
    }
}
fn call(id: &Value, name: &Value, args: &Value, encoded: bool) -> Result<ToolCall, LlmError> {
    let id = string(id)?;
    let name = string(name)?;
    if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) || !valid_name(name) {
        return Err(invalid());
    }
    let arguments = if encoded {
        serde_json::from_str::<Value>(string(args)?).map_err(|_| invalid())?
    } else {
        args.clone()
    };
    if !arguments.is_object() || arguments.to_string().len() > 16 * 1024 {
        return Err(invalid());
    }
    Ok(ToolCall {
        id: id.into(),
        name: name.into(),
        arguments_json: arguments.to_string(),
    })
}
pub(super) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}
pub(super) fn string(value: &Value) -> Result<&str, LlmError> {
    value.as_str().ok_or_else(invalid)
}
pub(super) fn array(value: &Value) -> Result<&Vec<Value>, LlmError> {
    value.as_array().ok_or_else(invalid)
}
