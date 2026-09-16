//! 上传参考 WAV 的严格格式、时长和静音校验。

use meowlive_application::ports::storage::ResourceStoreError;
use meowlive_domain::resources::ReferenceAudioMetadata;

use super::{MAX_REFERENCE_BYTES, store_error};

pub(super) fn validate_reference_wav(
    wav: &[u8],
) -> Result<ReferenceAudioMetadata, ResourceStoreError> {
    if wav.len() > MAX_REFERENCE_BYTES {
        return Err(store_error("参考 WAV 不能超过 2 MiB"));
    }
    let audio =
        crate::speech::wav::decode_wav(wav).map_err(|_| store_error("参考 WAV 格式损坏"))?;
    if !matches!(audio.channels, 1 | 2) || !(8_000..=48_000).contains(&audio.sample_rate) {
        return Err(store_error(
            "参考 WAV 必须是 8 至 48 kHz 的单声道或双声道 PCM16",
        ));
    }
    if !audio.samples.iter().any(|sample| *sample != 0) {
        return Err(store_error("参考 WAV 必须包含完整且非静音的音频"));
    }
    let frames = audio.samples.len() as u64 / u64::from(audio.channels);
    let sample_rate = u64::from(audio.sample_rate);
    if frames < sample_rate * 3 || frames > sample_rate * 10 {
        return Err(store_error("参考 WAV 时长必须在 3 至 10 秒之间"));
    }
    let duration_ms = u32::try_from(frames.saturating_mul(1000) / sample_rate)
        .map_err(|_| store_error("参考 WAV 时长无效"))?;
    ReferenceAudioMetadata::new(duration_ms, audio.sample_rate, audio.channels)
        .map_err(|_| store_error("参考 WAV 音频信息无效"))
}
