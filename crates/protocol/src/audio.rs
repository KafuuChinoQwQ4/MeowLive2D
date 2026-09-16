//! 音频帧头、采样格式、发言标识、代次、分片序号和结束标记的传输契约。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_AUDIO_SAMPLES: usize = 16_384;
pub const MAX_UTTERANCE_ID_BYTES: usize = 128;
const HEADER_BYTES: usize = 22;
pub const MAX_AUDIO_FRAME_BYTES: usize =
    HEADER_BYTES + MAX_UTTERANCE_ID_BYTES + MAX_AUDIO_SAMPLES * 2;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioFormat {
    pub fn validate(&self) -> Result<(), String> {
        if !(8_000..=96_000).contains(&self.sample_rate) || !(1..=2).contains(&self.channels) {
            return Err("PCM must use 8000..96000 Hz and one or two channels".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct AudioChunk {
    pub utterance_id: String,
    pub generation: u32,
    pub sequence: u32,
    pub samples: Vec<i16>,
    pub end: bool,
}

impl AudioChunk {
    /// MLPC, version:u16, flags:u16, generation:u32, sequence:u32,
    /// id_bytes:u16, sample_count:u32, UTF-8 identity, signed PCM16 samples.
    /// Every integer is little endian. Only bit zero of flags is defined (end).
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let id = self.utterance_id.as_bytes();
        if id.is_empty()
            || id.len() > MAX_UTTERANCE_ID_BYTES
            || self.samples.len() > MAX_AUDIO_SAMPLES
        {
            return Err("audio identity or payload exceeds frame bounds".into());
        }
        let mut frame = Vec::with_capacity(HEADER_BYTES + id.len() + self.samples.len() * 2);
        frame.extend_from_slice(b"MLPC");
        frame.extend_from_slice(&crate::PROTOCOL_VERSION.to_le_bytes());
        frame.extend_from_slice(&u16::from(self.end).to_le_bytes());
        frame.extend_from_slice(&self.generation.to_le_bytes());
        frame.extend_from_slice(&self.sequence.to_le_bytes());
        frame.extend_from_slice(&(id.len() as u16).to_le_bytes());
        frame.extend_from_slice(&(self.samples.len() as u32).to_le_bytes());
        frame.extend_from_slice(id);
        for sample in &self.samples {
            frame.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(frame)
    }

    pub fn decode(frame: &[u8]) -> Result<Self, String> {
        if !(HEADER_BYTES..=MAX_AUDIO_FRAME_BYTES).contains(&frame.len()) || &frame[..4] != b"MLPC"
        {
            return Err("invalid audio frame header or size".into());
        }
        let u16_at = |offset| u16::from_le_bytes([frame[offset], frame[offset + 1]]);
        let u32_at = |offset| {
            u32::from_le_bytes([
                frame[offset],
                frame[offset + 1],
                frame[offset + 2],
                frame[offset + 3],
            ])
        };
        if u16_at(4) != crate::PROTOCOL_VERSION || u16_at(6) > 1 {
            return Err("unsupported audio protocol version or flags".into());
        }
        let id_len = usize::from(u16_at(16));
        let sample_count = u32_at(18) as usize;
        if id_len == 0
            || id_len > MAX_UTTERANCE_ID_BYTES
            || sample_count > MAX_AUDIO_SAMPLES
            || frame.len() != HEADER_BYTES + id_len + sample_count * 2
        {
            return Err("invalid audio frame payload bounds".into());
        }
        let utterance_id = std::str::from_utf8(&frame[HEADER_BYTES..HEADER_BYTES + id_len])
            .map_err(|_| "audio identity is not UTF-8")?
            .to_owned();
        let samples = frame[HEADER_BYTES + id_len..]
            .chunks_exact(2)
            .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
            .collect();
        Ok(Self {
            utterance_id,
            generation: u32_at(8),
            sequence: u32_at(12),
            samples,
            end: u16_at(6) == 1,
        })
    }
}
