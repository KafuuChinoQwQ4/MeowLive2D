//! 音频设备边界；只有设备确认消费结束才允许完成回执。

use meowlive_protocol::audio::AudioFormat;

pub mod conversion;
pub mod meter;
mod simulated;
pub mod timing;
pub use simulated::SimulatedBackend;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::DeviceBackend;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendEvent {
    Started,
    Completed,
    Failed(String),
}

pub trait AudioBackend {
    fn start(&mut self, format: AudioFormat) -> Result<(), String>;
    fn push(&mut self, samples: &[i16]) -> Result<(), String>;
    fn finish(&mut self) -> Result<(), String>;
    fn stop(&mut self);
    fn poll(&mut self) -> Vec<BackendEvent>;
    /// Normalized RMS of samples scheduled at the current device playback time.
    fn output_level(&mut self) -> f32 {
        0.0
    }
}

#[cfg(not(windows))]
pub struct DeviceBackend;

#[cfg(not(windows))]
impl DeviceBackend {
    pub fn new(_: usize) -> Result<Self, String> {
        Err("audio device backend is supported only on Windows; --simulate explicitly enables silent test playback".into())
    }
}

#[cfg(not(windows))]
impl AudioBackend for DeviceBackend {
    fn start(&mut self, _: AudioFormat) -> Result<(), String> {
        Err("Windows audio device is unavailable".into())
    }
    fn push(&mut self, _: &[i16]) -> Result<(), String> {
        Err("Windows audio device is unavailable".into())
    }
    fn finish(&mut self) -> Result<(), String> {
        Err("Windows audio device is unavailable".into())
    }
    fn stop(&mut self) {}
    fn poll(&mut self) -> Vec<BackendEvent> {
        Vec::new()
    }
}
