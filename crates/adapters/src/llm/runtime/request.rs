use super::{ApiFormat, ValidatedConfig, invalid, output};
use meowlive_application::ports::{
    llm::{DecisionRequest, LlmError},
    llm_runtime::ModelOptions,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub(super) fn payload(
    config: &ValidatedConfig,
    format: ApiFormat,
    request: &DecisionRequest,
    options: &ModelOptions,
) -> Result<Value, LlmError> {
    if options.tools.len() > 16
        || options.exchanges.len() > 3
        || options
            .environment
            .as_ref()
            .is_some_and(|v| v.len() > 16 * 1024)
    {
        return Err(invalid());
    }
    let prompt = crate::llm::prompt::build_runtime_prompt(request)?;
    let system = prompt.system;
    let mut user: Value = serde_json::from_str(&prompt.user).map_err(|_| invalid())?;
    let field = if request.events.is_empty() {
        "recent_speeches"
    } else {
        "history"
    };
    let history = user
        .as_object_mut()
        .ok_or_else(invalid)?
        .remove(field)
        .unwrap_or_else(|| json!([]));
    let history =
        json!({field:history,"status":"completed interactions; never pending events"}).to_string();
    if let Some(environment) = &options.environment {
        user["environment"] = json!(environment);
    }
    let mut definitions = options.tools.iter().collect::<Vec<_>>();
    definitions.sort_by(|a, b| a.name.cmp(&b.name));
    let mut names = HashSet::new();
    let mut tools = Vec::new();
    for tool in definitions {
        if !output::valid_name(&tool.name)
            || !names.insert(&tool.name)
            || tool.description.len() > 2048
            || tool.parameters_json.len() > 16 * 1024
        {
            return Err(invalid());
        }
        let schema: Value = serde_json::from_str(&tool.parameters_json).map_err(|_| invalid())?;
        if !schema.is_object() || schema["type"] != "object" {
            return Err(invalid());
        }
        tools.push(match format {
            ApiFormat::OpenaiChat=>json!({"type":"function","function":{"name":tool.name,"description":tool.description,"parameters":schema}}),
            // Responses defaults to strict schemas; opt out because application
            // tools use ordinary JSON Schema with optional properties.
            ApiFormat::OpenaiResponses=>json!({"type":"function","name":tool.name,"description":tool.description,"parameters":schema,"strict":false}),
            ApiFormat::AnthropicMessages=>json!({"name":tool.name,"description":tool.description,"input_schema":schema}),
            ApiFormat::GeminiGenerateContent=>json!({"name":tool.name,"description":tool.description,"parametersJsonSchema":schema}),
        });
    }
    let mut body = match format {
        ApiFormat::OpenaiChat => {
            json!({"model":config.model,"max_tokens":config.max_tokens,"stream":options.stream,"messages":[{"role":"system","content":system},{"role":"user","content":history},{"role":"user","content":user.to_string()}]})
        }
        ApiFormat::OpenaiResponses => {
            json!({"model":config.model,"max_output_tokens":config.max_tokens,"store":false,"stream":options.stream,"input":[{"role":"system","content":system},{"role":"user","content":history},{"role":"user","content":user.to_string()}]})
        }
        ApiFormat::AnthropicMessages => {
            let mut system_block = json!({"type":"text","text":system});
            let mut history_block = json!({"type":"text","text":history});
            if options.cache_enabled {
                let cache = json!({"type":"ephemeral"});
                system_block["cache_control"] = cache.clone();
                history_block["cache_control"] = cache.clone();
                if let Some(tool) = tools.last_mut() {
                    tool["cache_control"] = cache;
                }
            }
            json!({"model":config.model,"max_tokens":config.max_tokens,"stream":options.stream,"system":[system_block],"messages":[{"role":"user","content":[history_block,{"type":"text","text":user.to_string()}]}]})
        }
        ApiFormat::GeminiGenerateContent => {
            json!({"systemInstruction":{"parts":[{"text":system}]},"generationConfig":{"maxOutputTokens":config.max_tokens},"contents":[{"role":"user","parts":[{"text":history},{"text":user.to_string()}]}]})
        }
    };
    if !tools.is_empty() {
        body["tools"] = if format == ApiFormat::GeminiGenerateContent {
            json!([{"functionDeclarations":tools}])
        } else {
            json!(tools)
        };
    }
    if config.json_mode {
        match format {
            ApiFormat::OpenaiChat => body["response_format"] = json!({"type":"json_object"}),
            ApiFormat::OpenaiResponses => body["text"] = json!({"format":{"type":"json_object"}}),
            // Gemini models differ on combining JSON MIME mode and tools. The
            // trusted system prompt plus final domain validation remains enforced.
            ApiFormat::GeminiGenerateContent if options.tools.is_empty() => {
                body["generationConfig"]["responseMimeType"] = json!("application/json")
            }
            _ => {}
        }
    }
    if options.stream && format == ApiFormat::OpenaiChat {
        body["stream_options"] = json!({"include_usage":true});
    }
    if options.tool_choice_none && !options.tools.is_empty() {
        match format {
            ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses => {
                body["tool_choice"] = json!("none")
            }
            ApiFormat::AnthropicMessages => body["tool_choice"] = json!({"type":"none"}),
            ApiFormat::GeminiGenerateContent => {
                body["toolConfig"] = json!({"functionCallingConfig":{"mode":"NONE"}})
            }
        }
    }
    if matches!(format, ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses)
        && official_openai(config)
    {
        if options.cache_enabled {
            let hash = Sha256::digest(
                format!("{}\n{}\n{}", config.model, system, json!(tools)).as_bytes(),
            );
            body["prompt_cache_key"] = json!(format!("meowlive-{:x}", hash));
        }
        if format == ApiFormat::OpenaiResponses {
            body["include"] = json!(["reasoning.encrypted_content"]);
        }
    }
    for exchange in &options.exchanges {
        if exchange.continuation.len() > config.max_response_bytes || exchange.results.len() > 4 {
            return Err(invalid());
        }
        let continuation: Value =
            serde_json::from_str(&exchange.continuation).map_err(|_| invalid())?;
        let normalized = output::parse(
            format,
            &output::continuation_response(format, continuation.clone()),
            &options.tools,
        )?;
        if normalized.calls.len() != exchange.results.len() {
            return Err(invalid());
        }
        let mut result_ids = HashSet::new();
        for result in &exchange.results {
            if result.content.len() > 32 * 1024
                || !result_ids.insert(&result.call_id)
                || !normalized
                    .calls
                    .iter()
                    .any(|call| call.id == result.call_id && call.name == result.name)
            {
                return Err(invalid());
            }
        }
        let key = match format {
            ApiFormat::OpenaiChat | ApiFormat::AnthropicMessages => "messages",
            ApiFormat::OpenaiResponses => "input",
            ApiFormat::GeminiGenerateContent => "contents",
        };
        let messages = body[key].as_array_mut().ok_or_else(invalid)?;
        if format == ApiFormat::OpenaiResponses {
            messages.extend(output::array(&continuation)?.clone());
        } else {
            messages.push(continuation.clone());
        }
        match format {
            ApiFormat::OpenaiChat=>for result in &exchange.results {messages.push(json!({"role":"tool","tool_call_id":result.call_id,"content":result.content}));},
            ApiFormat::OpenaiResponses=>for result in &exchange.results {messages.push(json!({"type":"function_call_output","call_id":result.call_id,"output":result.content}));},
            ApiFormat::AnthropicMessages=>messages.push(json!({"role":"user","content":exchange.results.iter().map(|r|json!({"type":"tool_result","tool_use_id":r.call_id,"content":r.content,"is_error":r.is_error})).collect::<Vec<_>>()})),
            ApiFormat::GeminiGenerateContent=>{
                let parts=output::array(&continuation["parts"])?;
                let results=normalized.calls.iter().map(|call| {
                    let r = exchange.results.iter().find(|r| r.call_id == call.id).ok_or_else(invalid)?;
                    let mut function=json!({"name":r.name,"response":{if r.is_error {"error"} else {"result"}:r.content}});
                    // Older Gemini responses omit IDs. Synthetic local IDs must
                    // never be sent as if the provider had issued them.
                    if parts.iter().any(|p|p["functionCall"]["id"].as_str()==Some(&r.call_id)) {function["id"]=json!(r.call_id);}
                    Ok(json!({"functionResponse":function}))
                }).collect::<Result<Vec<_>, LlmError>>()?;messages.push(json!({"role":"user","parts":results}));
            },
        }
    }
    crate::llm::reasoning::apply_reasoning(
        &mut body,
        &options.reasoning_provider,
        format,
        &config.model,
        options.reasoning_effort,
        config.max_tokens,
    )?;
    if body.to_string().len() > 1024 * 1024 {
        return Err(invalid());
    }
    Ok(body)
}
fn official_openai(config: &ValidatedConfig) -> bool {
    config.endpoint.scheme() == "https"
        && config.endpoint.host_str() == Some("api.openai.com")
        && config.endpoint.port_or_known_default() == Some(443)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_cache_key_is_stable_and_never_sent_to_other_hosts() {
        let cfg = |url: &str| {
            ValidatedConfig::new_for(
                crate::llm::config::LlmConfig {
                    base_url: url.into(),
                    model: "test".into(),
                    api_key: None,
                    timeout: std::time::Duration::from_secs(1),
                    max_response_bytes: 1024,
                    max_tokens: 64,
                    json_mode: false,
                },
                crate::llm::config::EndpointKind::OpenaiResponses,
            )
            .unwrap()
        };
        let request = DecisionRequest {
            persona: "cat".into(),
            topic: "one".into(),
            events: vec![],
            history: vec![],
            memory_context: vec![],
        };
        let mut opts = ModelOptions {
            cache_enabled: true,
            ..Default::default()
        };
        let a = payload(
            &cfg("https://api.openai.com/v1"),
            ApiFormat::OpenaiResponses,
            &request,
            &opts,
        )
        .unwrap();
        opts.environment = Some("dynamic environment".into());
        let b = payload(
            &cfg("https://api.openai.com/v1"),
            ApiFormat::OpenaiResponses,
            &request,
            &opts,
        )
        .unwrap();
        assert!(
            a["prompt_cache_key"]
                .as_str()
                .unwrap()
                .starts_with("meowlive-")
        );
        assert_eq!(a["prompt_cache_key"], b["prompt_cache_key"]);
        assert_eq!(a["input"][0], b["input"][0]);
        assert_eq!(a["input"][1], b["input"][1]);
        assert_ne!(a["input"][2], b["input"][2]);
        assert!(
            payload(
                &cfg("https://api.openai.com.example/v1"),
                ApiFormat::OpenaiResponses,
                &request,
                &opts
            )
            .unwrap()
            .get("prompt_cache_key")
            .is_none()
        );
        opts.cache_enabled = false;
        assert!(
            payload(
                &cfg("https://api.openai.com/v1"),
                ApiFormat::OpenaiResponses,
                &request,
                &opts
            )
            .unwrap()
            .get("prompt_cache_key")
            .is_none()
        );
    }
}
