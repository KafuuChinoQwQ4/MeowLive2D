//! 显式静音测试后端：模拟播放时长，不声称验证音频设备。

use super::{AudioBackend, BackendEvent, meter::SampleEnergy};
use meowlive_protocol::audio::AudioFormat;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub struct SimulatedBackend {
    maximum: usize,
    format: Option<AudioFormat>,
    deadline: Option<Instant>,
    ended: bool,
    started: bool,
    events: Vec<BackendEvent>,
    samples: VecDeque<i16>,
    sample_start: Option<Instant>,
    retired_samples: usize,
}

impl SimulatedBackend {
    pub fn new(maximum: usize) -> Self {
        Self {
            maximum,
            format: None,
            deadline: None,
            ended: false,
            started: false,
            events: Vec::new(),
            samples: VecDeque::with_capacity(maximum),
            sample_start: None,
            retired_samples: 0,
        }
    }

    fn retire_played_samples(&mut self, now: Instant, rate: f64) {
        let Some(start) = self.sample_start else {
            return;
        };
        let played = (now.saturating_duration_since(start).as_secs_f64() * rate) as usize;
        let count = played
            .saturating_sub(self.retired_samples)
            .min(self.samples.len());
        self.samples.drain(..count);
        self.retired_samples += count;
    }
}

impl AudioBackend for SimulatedBackend {
    fn start(&mut self, format: AudioFormat) -> Result<(), String> {
        format.validate()?;
        self.stop();
        self.format = Some(format);
        Ok(())
    }

    fn push(&mut self, samples: &[i16]) -> Result<(), String> {
        let format = self.format.ok_or("no simulated playback is active")?;
        let now = Instant::now();
        let rate = f64::from(format.sample_rate) * f64::from(format.channels);
        let deadline = self.deadline.unwrap_or(now).max(now);
        let buffered = deadline.duration_since(now).as_secs_f64() * rate;
        if buffered + samples.len() as f64 > self.maximum as f64 {
            return Err("simulated audio buffer capacity exceeded".into());
        }
        if self.deadline.is_none_or(|time| now >= time) {
            self.samples.clear();
            self.sample_start = Some(deadline);
            self.retired_samples = 0;
        } else {
            self.retire_played_samples(now, rate);
        }
        self.samples.extend(samples);
        if !samples.is_empty() && !self.started {
            self.events.push(BackendEvent::Started);
            self.started = true;
        }
        self.deadline = Some(deadline + Duration::from_secs_f64(samples.len() as f64 / rate));
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        self.ended = true;
        Ok(())
    }

    fn stop(&mut self) {
        self.format = None;
        self.deadline = None;
        self.ended = false;
        self.started = false;
        self.events.clear();
        self.samples.clear();
        self.sample_start = None;
        self.retired_samples = 0;
    }

    fn poll(&mut self) -> Vec<BackendEvent> {
        let mut events = std::mem::take(&mut self.events);
        if self.ended && self.deadline.is_some_and(|time| Instant::now() >= time) {
            events.push(BackendEvent::Completed);
            self.stop();
        }
        events
    }

    fn output_level(&mut self) -> f32 {
        let Some(format) = self.format else {
            return 0.0;
        };
        let now = Instant::now();
        let rate = f64::from(format.sample_rate) * f64::from(format.channels);
        self.retire_played_samples(now, rate);
        if self.deadline.is_none_or(|time| now >= time) {
            return 0.0;
        }
        // Only inspect the current 10 ms playback window. Future network chunks
        // remain PCM in the bounded queue until their simulated playback time.
        let window = (format.sample_rate as usize / 100) * usize::from(format.channels);
        let remaining = window - self.retired_samples % window;
        let mut energy = SampleEnergy::default();
        for &sample in self.samples.iter().take(remaining) {
            energy.add(f32::from(sample) / 32768.0);
        }
        energy.rms()
    }
}
