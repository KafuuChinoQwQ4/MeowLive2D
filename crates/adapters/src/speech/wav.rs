//! 严格校验完整 RIFF/WAV 容器，并解码可用于桌面音频传输的 PCM16。

use std::io::Cursor;

use meowlive_application::ports::speech::{PcmAudio, SynthesisError};

pub fn decode_wav(bytes: &[u8]) -> Result<PcmAudio, SynthesisError> {
    validate_container(bytes)?;
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(|_| invalid_wav())?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err(SynthesisError::new(
            "speech engine WAV must contain signed PCM16",
        ));
    }
    if !matches!(spec.channels, 1 | 2) || !(8000..=96000).contains(&spec.sample_rate) {
        return Err(SynthesisError::new(
            "speech engine WAV has an unsupported audio format",
        ));
    }
    let samples = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid_wav())?;
    if samples.is_empty() || samples.len() % usize::from(spec.channels) != 0 {
        return Err(SynthesisError::new(
            "speech engine WAV must contain complete nonempty audio frames",
        ));
    }
    Ok(PcmAudio {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        samples,
    })
}

/// Hound stops at the declared data chunk. Validate the outer container as well
/// so a truncated trailing chunk or dishonest RIFF length cannot pass decoding.
fn validate_container(bytes: &[u8]) -> Result<(), SynthesisError> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(invalid_wav());
    }
    let declared_size = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| invalid_wav())?);
    if u64::from(declared_size) + 8 != bytes.len() as u64 {
        return Err(invalid_wav());
    }
    let mut offset = 12usize;
    let mut data_chunks = 0;
    let mut format_chunks = 0;
    while offset < bytes.len() {
        let header = bytes
            .get(offset..offset.saturating_add(8))
            .ok_or_else(invalid_wav)?;
        let size = u32::from_le_bytes(header[4..8].try_into().map_err(|_| invalid_wav())?) as usize;
        if &header[..4] == b"fmt " {
            format_chunks += 1;
            if size < 16 {
                return Err(invalid_wav());
            }
            let format = bytes.get(offset + 8..offset + 24).ok_or_else(invalid_wav)?;
            let channels = u16::from_le_bytes([format[2], format[3]]);
            let block_align = u16::from_le_bytes([format[12], format[13]]);
            if channels.checked_mul(2) != Some(block_align) {
                return Err(invalid_wav());
            }
        } else if &header[..4] == b"data" {
            data_chunks += 1;
            if size == 0 || size % 2 != 0 {
                return Err(invalid_wav());
            }
        }
        offset = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(size))
            .and_then(|value| value.checked_add(size % 2))
            .filter(|value| *value <= bytes.len())
            .ok_or_else(invalid_wav)?;
    }
    if data_chunks != 1 || format_chunks != 1 {
        return Err(invalid_wav());
    }
    Ok(())
}

fn invalid_wav() -> SynthesisError {
    SynthesisError::new("speech engine returned invalid or truncated WAV audio")
}
