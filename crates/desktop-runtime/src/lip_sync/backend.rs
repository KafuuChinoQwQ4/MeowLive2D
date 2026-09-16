//! 观察设备输出，发布最新嘴部值；停止与 Drop 同步复位且不等待网络。
use super::{LipSync, LipSyncConfig};
use crate::audio::{AudioBackend, BackendEvent};
use meowlive_protocol::audio::AudioFormat;
use std::time::Duration;
use tokio::{sync::watch, time::Instant};

pub struct LipSyncBackend<B: AudioBackend> {
    inner: B,
    envelope: LipSync,
    sender: watch::Sender<f64>,
    period: Duration,
    last_sample: Instant,
    active: bool,
}

impl<B: AudioBackend> LipSyncBackend<B> {
    pub fn new(
        inner: B,
        config: LipSyncConfig,
        sender: watch::Sender<f64>,
    ) -> Result<Self, String> {
        config.validate()?;
        let period = Duration::from_secs_f64(1.0 / f64::from(config.update_hz));
        sender.send_replace(0.0);
        Ok(Self {
            inner,
            envelope: LipSync::new(config)?,
            sender,
            period,
            last_sample: Instant::now(),
            active: false,
        })
    }

    fn reset(&mut self) {
        self.active = false;
        self.envelope.reset();
        self.last_sample = Instant::now();
        self.sender.send_replace(0.0);
    }

    fn failed<T>(&mut self, result: Result<T, String>) -> Result<T, String> {
        if result.is_err() {
            self.inner.stop();
            self.reset();
        }
        result
    }
}

impl<B: AudioBackend> AudioBackend for LipSyncBackend<B> {
    fn start(&mut self, format: AudioFormat) -> Result<(), String> {
        self.reset();
        let result = self.inner.start(format);
        self.active = result.is_ok();
        self.failed(result)
    }
    fn push(&mut self, samples: &[i16]) -> Result<(), String> {
        let result = self.inner.push(samples);
        self.failed(result)
    }
    fn finish(&mut self) -> Result<(), String> {
        let result = self.inner.finish();
        self.failed(result)
    }
    fn stop(&mut self) {
        self.inner.stop();
        self.reset();
    }
    fn poll(&mut self) -> Vec<BackendEvent> {
        let events = self.inner.poll();
        if events
            .iter()
            .any(|event| matches!(event, BackendEvent::Completed | BackendEvent::Failed(_)))
        {
            self.reset();
        } else {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_sample);
            if elapsed >= self.period {
                let value = if self.active {
                    self.envelope.sample(self.inner.output_level(), elapsed)
                } else {
                    0.0
                };
                // Re-publish equal values so the VTS actor can detect stalled producers.
                self.sender.send_replace(value);
                self.last_sample = now;
            }
        }
        events
    }
    fn output_level(&mut self) -> f32 {
        self.inner.output_level()
    }
}

impl<B: AudioBackend> Drop for LipSyncBackend<B> {
    fn drop(&mut self) {
        self.inner.stop();
        self.reset();
    }
}
