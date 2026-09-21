//! 与设备无关的重采样输出缓冲，用于校验实际排队 PCM 的内存边界。

use super::conversion::PcmConverter;
use meowlive_protocol::audio::AudioFormat;
use std::collections::VecDeque;

pub const MAX_INPUT_SAMPLES: usize = 16_777_216;
const MAX_OUTPUT_SAMPLES: usize = 33_554_432;

pub struct ConvertedBuffer {
    samples: VecDeque<f32>,
    maximum: usize,
}

impl ConvertedBuffer {
    pub fn new(
        maximum_input: usize,
        source: AudioFormat,
        target_rate: u32,
        target_channels: u16,
    ) -> Result<Self, String> {
        if !(1..=MAX_INPUT_SAMPLES).contains(&maximum_input) {
            return Err("invalid device buffer capacity".into());
        }
        PcmConverter::new(source, target_rate, target_channels)?;
        let frames = maximum_input.div_ceil(usize::from(source.channels));
        let maximum = ((frames as u64 * u64::from(target_rate))
            .div_ceil(u64::from(source.sample_rate)) as usize
            + 1)
        .checked_mul(usize::from(target_channels))
        .ok_or("device buffer size overflow")?;
        // Conversion can expand the configured input limit beyond 128 MiB.
        // Limit queued output, not the theoretical size of a future utterance.
        Ok(Self {
            samples: VecDeque::new(),
            maximum: maximum.min(MAX_OUTPUT_SAMPLES),
        })
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
    pub fn pop_front(&mut self) -> Option<f32> {
        self.samples.pop_front()
    }

    pub fn extend(&mut self, samples: Vec<f32>) -> Result<(), String> {
        if samples.len() > self.maximum.saturating_sub(self.samples.len()) {
            return Err("Windows device buffer capacity exceeded".into());
        }
        let required = self.samples.len() + samples.len();
        if required > self.samples.capacity() {
            // Grow geometrically without reserving more than 128 MiB of f32 PCM.
            let capacity = required
                .max(self.samples.capacity().saturating_mul(2))
                .min(MAX_OUTPUT_SAMPLES);
            self.samples
                .try_reserve_exact(capacity - self.samples.len())
                .map_err(|_| "cannot allocate Windows device audio buffer")?;
        }
        self.samples.extend(samples);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mono() -> AudioFormat {
        AudioFormat {
            sample_rate: 32_000,
            channels: 1,
        }
    }

    #[test]
    fn large_input_limit_does_not_reject_or_allocate_for_a_short_clip() {
        let mut buffer = ConvertedBuffer::new(16_777_216, mono(), 48_000, 2).unwrap();
        assert_eq!(
            buffer.samples.capacity(),
            0,
            "starting playback must not preallocate the maximum"
        );
        buffer.extend(vec![0.5; 960]).unwrap();
        assert_eq!(buffer.len(), 960);
        assert!(buffer.samples.capacity() < 2048);
    }

    #[test]
    fn converted_buffer_accepts_four_minutes_of_32khz_mono_as_48khz_stereo() {
        let mut buffer = ConvertedBuffer::new(16_777_216, mono(), 48_000, 2).unwrap();
        let mut converter = PcmConverter::new(mono(), 48_000, 2).unwrap();
        for _ in 0..240 {
            buffer
                .extend(converter.convert(&vec![16_384; 32_000]).unwrap())
                .unwrap();
        }
        assert_eq!(buffer.len(), 23_040_000);
        assert_eq!(buffer.pop_front(), Some(0.5));
        assert!(buffer.samples.capacity() <= 33_554_432);
    }

    #[test]
    fn actual_output_overflow_is_rejected_without_changing_queued_audio() {
        let mut buffer = ConvertedBuffer::new(16_777_216, mono(), 48_000, 2).unwrap();
        for _ in 0..32 {
            buffer.extend(vec![0.25; 1_048_576]).unwrap();
        }
        assert!(buffer.extend(vec![0.75]).is_err());
        assert_eq!(buffer.len(), 33_554_432);
        assert_eq!(buffer.pop_front(), Some(0.25));
        assert!(buffer.samples.capacity() <= 33_554_432);
    }

    #[test]
    fn output_remains_bounded_by_a_smaller_configured_input_limit() {
        let mut buffer = ConvertedBuffer::new(4, mono(), 48_000, 2).unwrap();
        buffer.extend(vec![0.5; 14]).unwrap();
        assert!(buffer.extend(vec![0.75]).is_err());
        assert_eq!(buffer.len(), 14);
    }
}
