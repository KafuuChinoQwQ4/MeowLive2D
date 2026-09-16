//! 按资源音色解析参考文件并委托 GPT-SoVITS，保留旧 default 音色回退。

use super::gpt_sovits::{GptSovits, GptSovitsConfig};
use meowlive_application::{
    ports::speech::{SpeechSynthesizer, SynthesisError, SynthesisFuture, SynthesisRequest},
    resources::{ResolvedVoice, ResourceLibrary},
};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug)]
pub struct ResourceSynthesizerConfig {
    pub base_url: String,
    pub timeout: Duration,
    pub max_audio_bytes: usize,
}

pub struct ResourceSynthesizer {
    default: Arc<dyn SpeechSynthesizer>,
    library: Arc<ResourceLibrary>,
    config: ResourceSynthesizerConfig,
}

impl ResourceSynthesizer {
    pub fn new(
        default: Arc<dyn SpeechSynthesizer>,
        library: Arc<ResourceLibrary>,
        config: ResourceSynthesizerConfig,
    ) -> Result<Self, SynthesisError> {
        let endpoint = reqwest::Url::parse(&config.base_url)
            .map_err(|_| SynthesisError::new("语音引擎地址无效"))?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || config.timeout.is_zero()
            || config.max_audio_bytes == 0
        {
            return Err(SynthesisError::new("资源音色合成配置无效"));
        }
        Ok(Self {
            default,
            library,
            config,
        })
    }
}

impl SpeechSynthesizer for ResourceSynthesizer {
    fn synthesize(&self, request: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async move {
            let library = self.library.clone();
            let voice_id = request.voice_id.clone();
            let resolved = tokio::task::spawn_blocking(move || library.resolve_voice(&voice_id))
                .await
                .map_err(|_| SynthesisError::new("音色解析任务未完成"))?
                .map_err(|error| SynthesisError::new(error.to_string()))?;
            match resolved {
                ResolvedVoice::Default => self.default.synthesize(request).await,
                ResolvedVoice::Uploaded {
                    reference_audio,
                    reference_text,
                    language,
                    ..
                } => {
                    let language = language.as_str().to_owned();
                    let synthesizer = GptSovits::new(GptSovitsConfig {
                        base_url: self.config.base_url.clone(),
                        reference_audio,
                        prompt_text: reference_text,
                        prompt_language: language.clone(),
                        text_language: language,
                        timeout: self.config.timeout,
                        max_audio_bytes: self.config.max_audio_bytes,
                    })?;
                    synthesizer
                        .synthesize(SynthesisRequest {
                            text: request.text,
                            voice_id: "default".into(),
                        })
                        .await
                }
            }
        })
    }
}
