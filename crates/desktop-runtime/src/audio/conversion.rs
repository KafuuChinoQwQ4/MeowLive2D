//! 网络线程上的有界 PCM 声道映射与相位连续的零阶保持重采样。
//! 首版用于匹配默认设备格式；高保真滤波重采样留待设备音质验收。

use meowlive_protocol::audio::AudioFormat;

pub struct PcmConverter {
    source: AudioFormat,
    target_rate: u32,
    target_channels: u16,
    phase: u64,
}

impl PcmConverter {
    pub fn new(
        source: AudioFormat,
        target_rate: u32,
        target_channels: u16,
    ) -> Result<Self, String> {
        source.validate()?;
        if !(8_000..=192_000).contains(&target_rate) || !(1..=2).contains(&target_channels) {
            return Err(
                "default output device must support 8000..192000 Hz and one or two channels".into(),
            );
        }
        Ok(Self {
            source,
            target_rate,
            target_channels,
            phase: 0,
        })
    }

    pub fn convert(&mut self, samples: &[i16]) -> Result<Vec<f32>, String> {
        if samples.len() % usize::from(self.source.channels) != 0 {
            return Err("PCM sample count is not aligned to source channels".into());
        }
        let count = samples.len() / usize::from(self.source.channels);
        let capacity = ((count as u64 * u64::from(self.target_rate) + self.phase)
            / u64::from(self.source.sample_rate)) as usize
            * usize::from(self.target_channels);
        let mut output = Vec::with_capacity(capacity);
        for frame in samples.chunks_exact(usize::from(self.source.channels)) {
            let left = f32::from(frame[0]) / 32768.0;
            let right = if frame.len() == 2 {
                f32::from(frame[1]) / 32768.0
            } else {
                left
            };
            self.phase += u64::from(self.target_rate);
            while self.phase >= u64::from(self.source.sample_rate) {
                self.phase -= u64::from(self.source.sample_rate);
                if self.target_channels == 1 {
                    output.push((left + right) * 0.5);
                } else {
                    output.extend_from_slice(&[left, right]);
                }
            }
        }
        Ok(output)
    }
}
