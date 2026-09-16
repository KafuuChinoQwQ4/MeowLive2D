//! 音色档案、参考素材和模型版本的关联；推理克隆与训练产物分别记录。

use crate::speech::SpeechValidationError;
use std::fmt;

/// A portable configuration key, never a path to reference audio or a model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceId(String);

impl VoiceId {
    pub fn new(value: impl Into<String>) -> Result<Self, SpeechValidationError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(SpeechValidationError::InvalidVoiceId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VoiceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
