//! GPT-SoVITS HTTP 适配入口：参考素材路径解析、合成参数映射与音频解码。

use std::time::Duration;

use meowlive_application::ports::speech::{
    PcmAudio, SpeechSynthesizer, SynthesisError, SynthesisFuture, SynthesisRequest,
};
use meowlive_domain::speech::SpeechText;
use reqwest::{Client, Url};
use serde::Serialize;

use super::wav::decode_wav;

const MAX_REQUEST_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug)]
pub struct GptSovitsConfig {
    pub base_url: String,
    /// Resolved by the engine host, which may have a different filesystem.
    pub reference_audio: String,
    pub prompt_text: String,
    pub prompt_language: String,
    pub text_language: String,
    pub timeout: Duration,
    pub max_audio_bytes: usize,
}

pub struct GptSovits {
    client: Client,
    endpoint: Url,
    config: GptSovitsConfig,
}

impl GptSovits {
    pub fn new(config: GptSovitsConfig) -> Result<Self, SynthesisError> {
        if config.base_url.len() > 4096 {
            return Err(SynthesisError::new(
                "speech engine endpoint exceeds the size limit",
            ));
        }
        let mut endpoint = Url::parse(&config.base_url).map_err(|_| {
            SynthesisError::new("speech engine endpoint must be an HTTP or HTTPS URL")
        })?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(SynthesisError::new(
                "speech engine endpoint must be an HTTP or HTTPS URL without credentials, query or fragment",
            ));
        }
        if config.reference_audio.trim().is_empty() {
            return Err(SynthesisError::new(
                "speech reference audio is not configured",
            ));
        }
        if config.reference_audio.len() > 4096 || config.prompt_text.len() > 8192 {
            return Err(SynthesisError::new(
                "speech reference configuration exceeds the request size limit",
            ));
        }
        if [&config.prompt_language, &config.text_language]
            .iter()
            .any(|language| {
                language.is_empty()
                    || language.len() > 32
                    || !language
                        .bytes()
                        .all(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            })
        {
            return Err(SynthesisError::new(
                "speech language configuration is invalid",
            ));
        }
        if config.timeout.is_zero() || config.max_audio_bytes == 0 {
            return Err(SynthesisError::new(
                "speech timeout and response size limit must be positive",
            ));
        }
        endpoint.set_path(&format!("{}/tts", endpoint.path().trim_end_matches('/')));
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.timeout)
            .connect_timeout(config.timeout)
            .build()
            .map_err(|_| {
                SynthesisError::new("could not initialize the speech engine HTTP client")
            })?;
        Ok(Self {
            client,
            endpoint,
            config,
        })
    }

    async fn synthesize_audio(
        &self,
        request: SynthesisRequest,
    ) -> Result<PcmAudio, SynthesisError> {
        if request.voice_id != "default" {
            return Err(SynthesisError::new(
                "only the configured default voice is available",
            ));
        }
        let text = SpeechText::broadcast(request.text)
            .map_err(|error| SynthesisError::new(error.to_string()))?;
        let payload = TtsRequest {
            text: text.as_str(),
            text_lang: &self.config.text_language,
            ref_audio_path: &self.config.reference_audio,
            prompt_text: &self.config.prompt_text,
            prompt_lang: &self.config.prompt_language,
            text_split_method: "cut5",
            media_type: "wav",
            streaming_mode: false,
            batch_size: 1,
        };
        let request = self
            .client
            .post(self.endpoint.clone())
            .header(reqwest::header::ACCEPT, "audio/wav")
            .json(&payload)
            .build()
            .map_err(|_| SynthesisError::new("could not encode the speech engine request"))?;
        let request_size = request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .map_or(0, <[u8]>::len);
        if request_size > MAX_REQUEST_BYTES {
            return Err(SynthesisError::new(
                "speech engine request exceeds the size limit",
            ));
        }
        let mut response = self
            .client
            .execute(request)
            .await
            .map_err(transport_error)?;
        if !response.status().is_success() {
            return Err(SynthesisError::new(format!(
                "speech engine returned HTTP {}",
                response.status().as_u16()
            )));
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.config.max_audio_bytes as u64)
        {
            return Err(SynthesisError::new(
                "speech engine audio exceeds the response size limit",
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|length| length > self.config.max_audio_bytes)
            {
                return Err(SynthesisError::new(
                    "speech engine audio exceeds the response size limit",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let audio = decode_wav(&bytes)?;
        if audio.samples.iter().all(|sample| *sample == 0) {
            return Err(SynthesisError::new(
                "语音引擎返回了静音音频，请核对参考录音、参考文本和语言是否一致，再重试合成",
            ));
        }
        Ok(audio)
    }
}

impl SpeechSynthesizer for GptSovits {
    fn synthesize(&self, request: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(self.synthesize_audio(request))
    }
}

#[derive(Serialize)]
struct TtsRequest<'a> {
    text: &'a str,
    text_lang: &'a str,
    ref_audio_path: &'a str,
    prompt_text: &'a str,
    prompt_lang: &'a str,
    text_split_method: &'a str,
    media_type: &'a str,
    streaming_mode: bool,
    batch_size: u8,
}

fn transport_error(error: reqwest::Error) -> SynthesisError {
    if error.is_timeout() {
        SynthesisError::new("speech engine request timed out")
    } else {
        SynthesisError::new("speech engine request failed")
    }
}
