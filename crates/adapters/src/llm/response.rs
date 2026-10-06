use std::collections::HashSet;

use meowlive_application::ports::llm::{AgentDecision, DecisionRequest, LlmError};
use meowlive_domain::speech::SpeechText;
use serde::Deserialize;

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
    finish_reason: String,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: Option<String>,
    #[serde(default, rename = "tool_calls", deserialize_with = "field_is_present")]
    has_tool_calls: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    reply_to: Vec<String>,
    text: Option<String>,
    topic: Option<String>,
}

pub(super) fn parse_decision(
    bytes: &[u8],
    request: &DecisionRequest,
) -> Result<AgentDecision, LlmError> {
    let completion: Completion = serde_json::from_slice(bytes).map_err(|_| invalid_response())?;
    if completion.choices.len() != 1 {
        return Err(invalid_response());
    }
    let choice = completion
        .choices
        .into_iter()
        .next()
        .ok_or_else(invalid_response)?;
    if choice.finish_reason != "stop" || choice.message.has_tool_calls {
        return Err(invalid_response());
    }
    let content = choice.message.content.ok_or_else(invalid_response)?;
    if content.trim().is_empty() {
        return Err(invalid_response());
    }
    parse_decision_content(&content, request)
}

pub(super) fn parse_decision_content(
    content: &str,
    request: &DecisionRequest,
) -> Result<AgentDecision, LlmError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(invalid_response());
    }
    let decision = decode_decision(content).ok_or_else(invalid_response)?;
    validate_decision(decision, request)
}

// Some compatibility gateways put reasoning in content instead of a separate
// reasoning field. Accept only a complete final JSON object on its own line;
// never turn surrounding prose into speech or repair a malformed JSON decision.
fn decode_decision(content: &str) -> Option<Decision> {
    fn decode(content: &str) -> Option<Decision> {
        let body = if let Some(fenced) = content
            .strip_prefix("```json")
            .or_else(|| content.strip_prefix("```"))
        {
            fenced
                .strip_prefix("\r\n")
                .or_else(|| fenced.strip_prefix('\n'))?
                .strip_suffix("```")?
                .trim()
        } else {
            content
        };
        serde_json::from_str(body).ok()
    }
    if let Some(decision) = decode(content) {
        return Some(decision);
    }
    // A bare JSON object/array must stand on its own, even if it is invalid.
    if content.starts_with(['{', '[']) {
        return None;
    }
    let mut offset = 0;
    for line in content.split_inclusive('\n') {
        let candidate = content[offset..].trim();
        if (line.trim_start().starts_with('{') || line.trim_start().starts_with("```"))
            && let Some(decision) = decode(candidate)
        {
            return Some(decision);
        }
        offset += line.len();
    }
    None
}

fn validate_decision(
    decision: Decision,
    request: &DecisionRequest,
) -> Result<AgentDecision, LlmError> {
    if decision.reply_to.len() > request.events.len() {
        return Err(invalid_response());
    }
    let allowed = request
        .events
        .iter()
        .map(|event| event.id.as_str())
        .collect::<HashSet<_>>();
    let mut unique = HashSet::with_capacity(decision.reply_to.len());
    if decision
        .reply_to
        .iter()
        .any(|id| !allowed.contains(id.as_str()) || !unique.insert(id.as_str()))
    {
        return Err(invalid_response());
    }

    let text = decision
        .text
        .map(|text| SpeechText::new(text).map(|valid| valid.as_str().to_owned()))
        .transpose()
        .map_err(|_| invalid_response())?;
    if text.is_none() && !decision.reply_to.is_empty() {
        return Err(invalid_response());
    }
    if text.is_some() && !request.events.is_empty() && decision.reply_to.is_empty() {
        return Err(invalid_response());
    }
    if request.events.is_empty() && !decision.reply_to.is_empty() {
        return Err(invalid_response());
    }
    if decision
        .topic
        .as_ref()
        .is_some_and(|topic| topic.chars().count() > 200 || topic.chars().any(invalid_control))
    {
        return Err(invalid_response());
    }

    Ok(AgentDecision {
        reply_to: decision.reply_to,
        text,
        topic: decision.topic,
    })
}

fn invalid_response() -> LlmError {
    LlmError::new("LLM model returned an invalid decision", false)
}

fn invalid_control(character: char) -> bool {
    character.is_control() && !matches!(character, '\n' | '\t')
}

fn field_is_present<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::de::IgnoredAny::deserialize(deserializer)?;
    Ok(true)
}
