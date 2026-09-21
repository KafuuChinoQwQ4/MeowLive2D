use meowlive_application::ports::llm::LlmError;
use serde_json::Value;

use super::{ApiFormat, AvailableModel, invalid_response};

pub(super) fn entries(body: &Value, format: ApiFormat) -> Result<&Vec<Value>, LlmError> {
    let field = if format == ApiFormat::GeminiGenerateContent {
        "models"
    } else {
        "data"
    };
    body.get(field)
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)
}

pub(super) fn parse_model(
    entry: &Value,
    format: ApiFormat,
) -> Result<Option<AvailableModel>, LlmError> {
    let (id_field, name_field) = match format {
        ApiFormat::GeminiGenerateContent => ("name", "displayName"),
        _ => ("id", "display_name"),
    };
    let raw_id = entry
        .get(id_field)
        .and_then(Value::as_str)
        .ok_or_else(invalid_response)?;
    let id = if format == ApiFormat::GeminiGenerateContent {
        raw_id
            .strip_prefix("models/")
            .ok_or_else(invalid_response)?
    } else {
        raw_id
    };
    if id.is_empty() || id.len() > 128 || id.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(invalid_response());
    }
    let name = optional_string(entry, name_field)?
        .filter(|name| {
            !name.trim().is_empty() && name.len() <= 256 && !name.chars().any(char::is_control)
        })
        .unwrap_or(id);
    if format == ApiFormat::GeminiGenerateContent {
        let methods = entry
            .get("supportedGenerationMethods")
            .and_then(Value::as_array)
            .ok_or_else(invalid_response)?;
        if methods.iter().any(|method| !method.is_string()) {
            return Err(invalid_response());
        }
        if !methods
            .iter()
            .any(|method| method.as_str() == Some("generateContent"))
        {
            return Ok(None);
        }
    }
    Ok(Some(AvailableModel {
        id: id.to_owned(),
        name: name.to_owned(),
    }))
}

pub(super) fn next_cursor(body: &Value, format: ApiFormat) -> Result<Option<String>, LlmError> {
    let cursor = match format {
        ApiFormat::GeminiGenerateContent => {
            optional_string(body, "nextPageToken")?.filter(|token| !token.is_empty())
        }
        ApiFormat::AnthropicMessages => {
            let more = match body.get("has_more") {
                Some(value) => value.as_bool().ok_or_else(invalid_response)?,
                None => false,
            };
            if more {
                Some(optional_string(body, "last_id")?.ok_or_else(invalid_response)?)
            } else {
                None
            }
        }
        ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses => {
            if let Some(more) = body.get("has_more") {
                if more.as_bool().ok_or_else(invalid_response)? {
                    return Err(LlmError::new(
                        "供应商模型目录使用了不受支持的分页，无法获取完整列表",
                        false,
                    ));
                }
            }
            None
        }
    };
    if cursor.is_some_and(|value| {
        value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control)
    }) {
        return Err(invalid_response());
    }
    Ok(cursor.map(str::to_owned))
}

fn optional_string<'a>(body: &'a Value, field: &str) -> Result<Option<&'a str>, LlmError> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_str().map(Some).ok_or_else(invalid_response),
    }
}
