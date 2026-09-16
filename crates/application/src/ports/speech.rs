//! 统一文本转语音请求、音频输出与取消能力；不让上层依赖引擎请求字段。

use std::{fmt, future::Future, pin::Pin};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesisRequest {
    pub text: String,
    pub voice_id: String,
}

/// Interleaved signed PCM samples. Adapters validate the format before returning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PcmAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesisError {
    pub message: String,
}

impl SynthesisError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SynthesisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SynthesisError {}

pub type SynthesisFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PcmAudio, SynthesisError>> + Send + 'a>>;

pub trait SpeechSynthesizer: Send + Sync {
    /// Dropping the returned future cancels the caller's interest in synthesis.
    fn synthesize(&self, request: SynthesisRequest) -> SynthesisFuture<'_>;
}
