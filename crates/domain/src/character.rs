//! 角色、人设与可用表情动作的能力映射。使用业务标识，不使用 VTS 原始消息。

use crate::resources::{ResourceValidationError, valid_name};
use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterMapping {
    intent: String,
    hotkey_id: String,
    fallback_hotkey_id: Option<String>,
    validated: bool,
}

impl CharacterMapping {
    pub fn new(
        intent: impl Into<String>,
        hotkey_id: impl Into<String>,
        fallback_hotkey_id: Option<String>,
    ) -> Result<Self, ResourceValidationError> {
        let intent = intent.into();
        let hotkey_id = hotkey_id.into();
        if !valid_text(&intent, 40) {
            return Err(ResourceValidationError::InvalidIntent);
        }
        if !valid_external_id(&hotkey_id)
            || fallback_hotkey_id
                .as_deref()
                .is_some_and(|value| !valid_external_id(value))
        {
            return Err(ResourceValidationError::InvalidHotkeyId);
        }
        Ok(Self {
            intent,
            hotkey_id,
            fallback_hotkey_id,
            validated: false,
        })
    }

    pub fn intent(&self) -> &str {
        &self.intent
    }

    pub fn hotkey_id(&self) -> &str {
        &self.hotkey_id
    }

    pub fn fallback_hotkey_id(&self) -> Option<&str> {
        self.fallback_hotkey_id.as_deref()
    }

    pub fn validated(&self) -> bool {
        self.validated
    }

    pub fn mark_validated(&mut self) {
        self.validated = true;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterProfile {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub voice_id: String,
    pub mouth_parameter: String,
    pub mappings: Vec<CharacterMapping>,
}

impl CharacterProfile {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        model_id: impl Into<String>,
        voice_id: impl Into<String>,
        mouth_parameter: impl Into<String>,
        mut mappings: Vec<CharacterMapping>,
    ) -> Result<Self, ResourceValidationError> {
        let profile = Self {
            id: id.into(),
            name: name.into(),
            model_id: model_id.into(),
            voice_id: voice_id.into(),
            mouth_parameter: mouth_parameter.into(),
            mappings: {
                for mapping in &mut mappings {
                    mapping.validated = false;
                }
                mappings
            },
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), ResourceValidationError> {
        if !valid_portable_id(&self.id) {
            return Err(ResourceValidationError::InvalidCharacterId);
        }
        if !valid_name(&self.name) {
            return Err(ResourceValidationError::InvalidName);
        }
        if !valid_external_id(&self.model_id) {
            return Err(ResourceValidationError::InvalidModelId);
        }
        if !valid_portable_id(&self.voice_id) {
            return Err(ResourceValidationError::InvalidVoiceId);
        }
        if !(4..=32).contains(&self.mouth_parameter.len())
            || !self
                .mouth_parameter
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(ResourceValidationError::InvalidMouthParameter);
        }
        if self.mappings.len() > 32 {
            return Err(ResourceValidationError::TooManyMappings);
        }
        let mut intents = HashSet::new();
        for mapping in &self.mappings {
            if !intents.insert(mapping.intent()) {
                return Err(ResourceValidationError::DuplicateIntent);
            }
        }
        Ok(())
    }

    pub fn mark_mapping_validated(&mut self, intent: &str) -> Result<(), ResourceValidationError> {
        let mapping = self
            .mappings
            .iter_mut()
            .find(|mapping| mapping.intent() == intent)
            .ok_or(ResourceValidationError::MappingNotFound)?;
        mapping.mark_validated();
        Ok(())
    }
}

fn valid_text(value: &str, maximum: usize) -> bool {
    let value = value.trim();
    !value.is_empty() && value.chars().count() <= maximum && !value.chars().any(char::is_control)
}

fn valid_external_id(value: &str) -> bool {
    valid_text(value, 128)
}

fn valid_portable_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}
