//! 发言标识、生成代次与音频格式语义，用于关联合成、取消和迟到数据。

use std::fmt;

use crate::voice::VoiceId;

/// Validated speech content, limited by Unicode scalar values rather than bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeechText(String);

impl SpeechText {
    pub const MAX_CHARS: usize = 500;

    pub fn new(text: impl Into<String>) -> Result<Self, SpeechValidationError> {
        let text = text.into();
        let text = text.trim();
        if text.is_empty() {
            return Err(SpeechValidationError::EmptyText);
        }
        if text.chars().count() > Self::MAX_CHARS {
            return Err(SpeechValidationError::TextTooLong {
                max: Self::MAX_CHARS,
            });
        }
        if text
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return Err(SpeechValidationError::InvalidText);
        }
        Ok(Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SpeechText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeechValidationError {
    EmptyText,
    TextTooLong { max: usize },
    InvalidText,
    InvalidVoiceId,
}

impl fmt::Display for SpeechValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyText => formatter.write_str("speech text must not be empty"),
            Self::TextTooLong { max } => {
                write!(
                    formatter,
                    "speech text must contain at most {max} characters"
                )
            }
            Self::InvalidText => formatter.write_str("speech text contains a control character"),
            Self::InvalidVoiceId => formatter.write_str(
                "voice ID must contain 1 to 64 ASCII letters, numbers, underscores or hyphens",
            ),
        }
    }
}

impl std::error::Error for SpeechValidationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeechStatus {
    Queued,
    Synthesizing,
    Ready,
    Playing,
    Completed,
    Cancelled,
    Failed,
    Unknown,
}

impl SpeechStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Failed | Self::Unknown
        )
    }
}

/// An executor receipt describes an observed event, not an arbitrary target state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeechReceiptStatus {
    Started,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeechTask {
    pub id: String,
    pub generation: u32,
    pub text: SpeechText,
    pub voice_id: VoiceId,
    pub status: SpeechStatus,
    pub error: Option<String>,
}
