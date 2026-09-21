//! SSE 帧跨 HTTP chunk / UTF-8 边界拼接；显式终止前不交付模型结果。
use super::{ApiFormat, Progress, invalid, limit, output, transport_error};
use meowlive_application::ports::llm::LlmError;
use serde_json::{Value, json};

// SSE repeats IDs, model names, event names and JSON structure per token.
// Keep protocol overhead separate from the configured assembled JSON budget.
pub(super) fn wire_limit(max_bytes: usize) -> usize {
    max_bytes.saturating_mul(8).min(8 * 1024 * 1024)
}

pub(super) async fn receive(
    response: &mut reqwest::Response,
    format: ApiFormat,
    max_bytes: usize,
    progress: &mut Progress<'_>,
) -> Result<Value, LlmError> {
    let mut decoder = Decoder::default();
    let mut state = State::new(format);
    let mut received = 0usize;
    let wire_limit = wire_limit(max_bytes);
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        let remaining = wire_limit.saturating_sub(received);
        if chunk.len() > remaining {
            // Record complete usage frames within the permitted prefix even
            // when the same network chunk crosses the response budget.
            let _ = decoder.push(&chunk[..remaining], |data| state.frame(data, progress));
            return Err(limit());
        }
        received += chunk.len();
        decoder.push(&chunk, |data| state.frame(data, progress))?;
    }
    if !decoder.complete() || !state.terminal {
        return Err(invalid());
    }
    if state.raw.to_string().len() > max_bytes {
        return Err(limit());
    }
    Ok(state.raw)
}

#[derive(Default)]
struct Decoder {
    pending: Vec<u8>,
    data: String,
}
impl Decoder {
    fn push(
        &mut self,
        bytes: &[u8],
        mut consume: impl FnMut(&str) -> Result<(), LlmError>,
    ) -> Result<(), LlmError> {
        self.pending.extend_from_slice(bytes);
        let mut consumed = 0;
        while let Some(offset) = self.pending[consumed..].iter().position(|b| *b == b'\n') {
            let end = consumed + offset;
            let line = std::str::from_utf8(&self.pending[consumed..end])
                .map_err(|_| invalid())?
                .trim_end_matches('\r');
            if line.is_empty() {
                if !self.data.is_empty() {
                    self.data.pop();
                    // Deliver each valid frame before decoding the next one:
                    // malformed bytes later in this HTTP chunk must not erase
                    // usage that the provider already reported.
                    consume(&std::mem::take(&mut self.data))?;
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
                self.data.push('\n');
            }
            consumed = end + 1;
        }
        self.pending.drain(..consumed);
        Ok(())
    }
    fn complete(&self) -> bool {
        self.data.is_empty() && self.pending.iter().all(u8::is_ascii_whitespace)
    }
}

struct State {
    format: ApiFormat,
    raw: Value,
    terminal: bool,
    started: bool,
    blocks: Vec<bool>,
    arguments: Vec<String>,
}
impl State {
    fn new(format: ApiFormat) -> Self {
        let raw = match format {
            ApiFormat::OpenaiChat => {
                json!({"choices":[{"message":{"role":"assistant","content":""},"finish_reason":null}]})
            }
            ApiFormat::OpenaiResponses => json!({"status":"in_progress","output":[]}),
            ApiFormat::AnthropicMessages => {
                json!({"type":"message","role":"assistant","content":[]})
            }
            ApiFormat::GeminiGenerateContent => {
                json!({"candidates":[{"content":{"role":"model","parts":[]}}]})
            }
        };
        Self {
            format,
            raw,
            terminal: false,
            started: false,
            blocks: vec![],
            arguments: vec![],
        }
    }
    fn frame(&mut self, data: &str, progress: &mut Progress<'_>) -> Result<(), LlmError> {
        if data == "[DONE]" {
            if self.format != ApiFormat::OpenaiChat
                || self.terminal
                || self.raw["choices"][0]["finish_reason"].is_null()
            {
                return Err(invalid());
            }
            self.terminal = true;
            return Ok(());
        }
        let event: Value = serde_json::from_str(data).map_err(|_| invalid())?;
        let usage_key = if self.format == ApiFormat::GeminiGenerateContent {
            "usageMetadata"
        } else {
            "usage"
        };
        if let Some(usage) = event.get(usage_key) {
            merge(&mut self.raw[usage_key], usage);
        }
        if let Some(usage) = event.get("message").and_then(|m| m.get("usage")) {
            merge(&mut self.raw["usage"], usage);
        }
        if let Some(usage) = event.get("response").and_then(|m| m.get("usage")) {
            merge(&mut self.raw["usage"], usage);
        }
        progress.usage(output::usage(self.format, &self.raw));
        if event.get("error").is_some() || event["type"] == "error" {
            return Err(invalid());
        }
        // Gemini may put usage in a separate final chunk after finishReason.
        if self.terminal
            && !(self.format == ApiFormat::GeminiGenerateContent
                && event
                    .get("candidates")
                    .is_none_or(|v| v.as_array().is_some_and(Vec::is_empty)))
        {
            return Err(invalid());
        }
        match self.format {
            ApiFormat::OpenaiChat => self.chat(&event, progress),
            ApiFormat::OpenaiResponses => self.responses(&event, progress),
            ApiFormat::AnthropicMessages => self.anthropic(&event, progress),
            ApiFormat::GeminiGenerateContent => self.gemini(&event, progress),
        }
    }
    fn chat(&mut self, event: &Value, progress: &mut Progress<'_>) -> Result<(), LlmError> {
        let choices = output::array(&event["choices"])?;
        if choices.is_empty() {
            return Ok(());
        }
        if choices.len() != 1 || choices[0]["index"] != 0 {
            return Err(invalid());
        }
        if !self.raw["choices"][0]["finish_reason"].is_null() {
            return Err(invalid());
        }
        let choice = &choices[0];
        let delta = &choice["delta"];
        if !delta.is_object() {
            return Err(invalid());
        }
        if delta
            .get("role")
            .is_some_and(|v| !v.is_null() && v != "assistant")
            || delta.get("refusal").is_some_and(|v| !v.is_null())
            || delta.get("function_call").is_some()
        {
            return Err(invalid());
        }
        let message = &mut self.raw["choices"][0]["message"];
        if let Some(text) = delta.get("content").filter(|v| !v.is_null()) {
            let text = output::string(text)?;
            append(&mut message["content"], text)?;
            progress.output(text);
        }
        for key in ["reasoning_content", "reasoning"] {
            if let Some(text) = delta.get(key).filter(|v| !v.is_null()) {
                let text = output::string(text)?;
                append(&mut message[key], text)?;
                if !text.is_empty() {
                    progress.first();
                }
            }
        }
        if let Some(calls) = delta.get("tool_calls").filter(|v| !v.is_null()) {
            if message.get("tool_calls").is_none() {
                message["tool_calls"] = json!([]);
            }
            let accumulated = message["tool_calls"].as_array_mut().ok_or_else(invalid)?;
            for call in output::array(calls)? {
                let index = index(&call["index"], 4)?;
                if index > accumulated.len() {
                    return Err(invalid());
                }
                if index == accumulated.len() {
                    accumulated.push(
                        json!({"id":"","type":"function","function":{"name":"","arguments":""}}),
                    );
                }
                if call.get("type").is_some_and(|v| v != "function") {
                    return Err(invalid());
                }
                if let Some(id) = call.get("id") {
                    append(&mut accumulated[index]["id"], output::string(id)?)?;
                }
                for key in ["name", "arguments"] {
                    if let Some(value) = call["function"].get(key) {
                        append(
                            &mut accumulated[index]["function"][key],
                            output::string(value)?,
                        )?;
                    }
                }
                progress.first();
            }
        }
        if let Some(reason) = choice.get("finish_reason").filter(|v| !v.is_null()) {
            output::string(reason)?;
            self.raw["choices"][0]["finish_reason"] = reason.clone();
        }
        Ok(())
    }
    fn responses(&mut self, event: &Value, progress: &mut Progress<'_>) -> Result<(), LlmError> {
        match event["type"].as_str() {
            Some("response.output_text.delta") => progress.output(output::string(&event["delta"])?),
            Some(
                "response.function_call_arguments.delta"
                | "response.reasoning_summary_text.delta"
                | "response.reasoning_text.delta",
            ) => {
                output::string(&event["delta"])?;
                progress.first();
            }
            Some("response.completed") => {
                if event["response"]["status"] != "completed" {
                    return Err(invalid());
                }
                self.raw = event["response"].clone();
                self.terminal = true;
            }
            Some(
                "response.incomplete"
                | "response.failed"
                | "response.refusal.delta"
                | "response.refusal.done",
            ) => return Err(invalid()),
            Some("response.output_item.added") => {
                if event["item"]["type"] == "function_call" {
                    progress.first();
                }
            }
            Some(_) => {}
            None => return Err(invalid()),
        }
        Ok(())
    }
    fn anthropic(&mut self, event: &Value, progress: &mut Progress<'_>) -> Result<(), LlmError> {
        match event["type"].as_str() {
            Some("message_start") => {
                if self.started
                    || event["message"]["type"] != "message"
                    || event["message"]["role"] != "assistant"
                    || !output::array(&event["message"]["content"])?.is_empty()
                {
                    return Err(invalid());
                }
                self.started = true;
            }
            Some("content_block_start") => {
                if !self.started {
                    return Err(invalid());
                }
                let index = index(&event["index"], 64)?;
                if index != self.blocks.len() {
                    return Err(invalid());
                }
                let block = event["content_block"].clone();
                match block["type"].as_str() {
                    Some("text") => progress.output(output::string(&block["text"])?),
                    Some("tool_use" | "thinking" | "redacted_thinking") => progress.first(),
                    _ => return Err(invalid()),
                }
                self.raw["content"]
                    .as_array_mut()
                    .ok_or_else(invalid)?
                    .push(block);
                self.blocks.push(true);
                self.arguments.push(String::new());
            }
            Some("content_block_delta") => {
                let index = index(&event["index"], 64)?;
                if self.blocks.get(index) != Some(&true) {
                    return Err(invalid());
                }
                let delta = &event["delta"];
                let block = &mut self.raw["content"][index];
                match delta["type"].as_str() {
                    Some("text_delta") if block["type"] == "text" => {
                        let text = output::string(&delta["text"])?;
                        append(&mut block["text"], text)?;
                        progress.output(text);
                    }
                    Some("thinking_delta") if block["type"] == "thinking" => {
                        append(&mut block["thinking"], output::string(&delta["thinking"])?)?;
                        progress.first();
                    }
                    Some("signature_delta") if block["type"] == "thinking" => append(
                        &mut block["signature"],
                        output::string(&delta["signature"])?,
                    )?,
                    Some("input_json_delta") if block["type"] == "tool_use" => {
                        self.arguments[index].push_str(output::string(&delta["partial_json"])?);
                        progress.first();
                    }
                    _ => return Err(invalid()),
                }
            }
            Some("content_block_stop") => {
                let index = index(&event["index"], 64)?;
                if self.blocks.get(index) != Some(&true) {
                    return Err(invalid());
                }
                self.blocks[index] = false;
                if self.raw["content"][index]["type"] == "tool_use"
                    && !self.arguments[index].is_empty()
                {
                    let input: Value =
                        serde_json::from_str(&self.arguments[index]).map_err(|_| invalid())?;
                    if !input.is_object() {
                        return Err(invalid());
                    }
                    self.raw["content"][index]["input"] = input;
                }
            }
            Some("message_delta") => {
                if !self.started || self.blocks.contains(&true) {
                    return Err(invalid());
                }
                if let Some(reason) = event["delta"].get("stop_reason").filter(|v| !v.is_null()) {
                    self.raw["stop_reason"] = reason.clone();
                }
            }
            Some("message_stop") => {
                if !self.started || self.blocks.contains(&true) || self.raw["stop_reason"].is_null()
                {
                    return Err(invalid());
                }
                self.terminal = true;
            }
            Some(_) => {}
            None => return Err(invalid()),
        }
        Ok(())
    }
    fn gemini(&mut self, event: &Value, progress: &mut Progress<'_>) -> Result<(), LlmError> {
        if event["promptFeedback"].get("blockReason").is_some() {
            return Err(invalid());
        }
        let Some(candidates) = event.get("candidates") else {
            return Ok(());
        };
        let candidates = output::array(candidates)?;
        if candidates.is_empty() {
            return Ok(());
        }
        if candidates.len() != 1 || candidates[0].get("index").is_some_and(|v| v != 0) {
            return Err(invalid());
        }
        let candidate = &candidates[0];
        if let Some(content) = candidate.get("content") {
            if content.get("role").is_some_and(|v| v != "model") {
                return Err(invalid());
            }
            for part in output::array(&content["parts"])? {
                if let Some(text) = part.get("text") {
                    let text = output::string(text)?;
                    if part["thought"] != true {
                        progress.output(text);
                    } else if !text.is_empty() {
                        progress.first();
                    }
                }
                if part.get("functionCall").is_some() {
                    progress.first();
                }
                self.raw["candidates"][0]["content"]["parts"]
                    .as_array_mut()
                    .ok_or_else(invalid)?
                    .push(part.clone());
            }
        }
        if let Some(reason) = candidate.get("finishReason") {
            self.raw["candidates"][0]["finishReason"] = reason.clone();
            self.terminal = true;
        }
        Ok(())
    }
}
fn append(target: &mut Value, text: &str) -> Result<(), LlmError> {
    if target.is_null() {
        *target = json!("");
    }
    let mut existing = output::string(target)?.to_owned();
    existing.push_str(text);
    *target = json!(existing);
    Ok(())
}
fn index(value: &Value, limit: usize) -> Result<usize, LlmError> {
    let value = value.as_u64().ok_or_else(invalid)?;
    if value >= limit as u64 {
        return Err(invalid());
    }
    Ok(value as usize)
}
fn merge(target: &mut Value, value: &Value) {
    if let Some(object) = value.as_object() {
        if !target.is_object() {
            *target = json!({});
        }
        for (key, value) in object {
            merge(&mut target[key], value);
        }
    } else if !value.is_null() {
        *target = value.clone();
    }
}
