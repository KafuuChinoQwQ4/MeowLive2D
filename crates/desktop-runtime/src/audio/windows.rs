//! CPAL/WASAPI 默认输出设备；网络与重采样均在设备回调之外。

use super::{
    AudioBackend, BackendEvent,
    buffer::{ConvertedBuffer, MAX_INPUT_SAMPLES},
    conversion::PcmConverter,
    meter::{OutputMeter, SampleEnergy},
    timing::DeviceClock,
};
use cpal::{
    FromSample, SizedSample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use meowlive_protocol::audio::AudioFormat;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

struct Buffer {
    samples: ConvertedBuffer,
    ended: bool,
    clock: DeviceClock,
    meter: OutputMeter,
}

pub struct DeviceBackend {
    maximum: usize,
    stream: Option<cpal::Stream>,
    buffer: Option<Arc<Mutex<Buffer>>>,
    converter: Option<PcmConverter>,
    failed: Arc<AtomicBool>,
}

impl DeviceBackend {
    pub fn new(maximum: usize) -> Result<Self, String> {
        if !(1..=MAX_INPUT_SAMPLES).contains(&maximum) {
            return Err("invalid device buffer capacity".into());
        }
        cpal::default_host()
            .default_output_device()
            .ok_or("no default Windows audio output device")?;
        Ok(Self {
            maximum,
            stream: None,
            buffer: None,
            converter: None,
            failed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn build<T: SizedSample + FromSample<f32>>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        shared: Arc<Mutex<Buffer>>,
        failed: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, String> {
        let channels = usize::from(config.channels);
        let rate = config.sample_rate.0;
        device
            .build_output_stream(
                config,
                move |output: &mut [T], info: &cpal::OutputCallbackInfo| {
                    let now = Instant::now();
                    // Never block the audio thread. Silence represents a temporary
                    // underrun, and consumes no queued PCM or playback clock.
                    output.fill(T::from_sample(0.0));
                    let Ok(mut buffer) = shared.try_lock() else {
                        return;
                    };
                    let consumed = output.len().min(buffer.samples.len());
                    let consumed = consumed - consumed % channels;
                    let stamp = info.timestamp();
                    let latency = stamp
                        .playback
                        .duration_since(&stamp.callback)
                        .unwrap_or_default();
                    let window = (rate as usize / 100).max(1) * channels;
                    for (index, block) in output[..consumed].chunks_mut(window).enumerate() {
                        let mut energy = SampleEnergy::default();
                        for slot in block.iter_mut() {
                            let sample = buffer.samples.pop_front().unwrap_or(0.0);
                            energy.add(sample);
                            *slot = T::from_sample(sample);
                        }
                        let offset = Duration::from_secs_f64(
                            (index * window / channels) as f64 / f64::from(rate),
                        );
                        buffer.meter.record(
                            now + offset,
                            latency,
                            block.len() / channels,
                            rate,
                            energy,
                        );
                    }
                    // The unfilled tail is a distinct silent interval, so an
                    // underrun never extends the preceding block's mouth level.
                    let offset =
                        Duration::from_secs_f64((consumed / channels) as f64 / f64::from(rate));
                    buffer.meter.record(
                        now + offset,
                        latency,
                        (output.len() - consumed) / channels,
                        rate,
                        SampleEnergy::default(),
                    );
                    if consumed > 0 {
                        buffer
                            .clock
                            .submitted(now, latency, consumed / channels, rate);
                    }
                },
                move |_| {
                    failed.store(true, Ordering::Release);
                },
                None,
            )
            .map_err(|error| format!("cannot build Windows audio stream: {error}"))
    }
}

impl AudioBackend for DeviceBackend {
    fn start(&mut self, format: AudioFormat) -> Result<(), String> {
        self.stop();
        format.validate()?;
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no default Windows audio output device")?;
        let supported = device
            .default_output_config()
            .map_err(|error| error.to_string())?;
        let config: cpal::StreamConfig = supported.clone().into();
        let converter = PcmConverter::new(format, config.sample_rate.0, config.channels)?;
        let buffer = Arc::new(Mutex::new(Buffer {
            samples: ConvertedBuffer::new(
                self.maximum,
                format,
                config.sample_rate.0,
                config.channels,
            )?,
            ended: false,
            clock: DeviceClock::default(),
            meter: OutputMeter::default(),
        }));
        self.failed = Arc::new(AtomicBool::new(false));
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build::<f32>(&device, &config, buffer.clone(), self.failed.clone())
            }
            cpal::SampleFormat::I16 => {
                Self::build::<i16>(&device, &config, buffer.clone(), self.failed.clone())
            }
            cpal::SampleFormat::U16 => {
                Self::build::<u16>(&device, &config, buffer.clone(), self.failed.clone())
            }
            other => Err(format!("unsupported Windows output sample format: {other}")),
        }?;
        stream
            .play()
            .map_err(|error| format!("cannot start Windows audio stream: {error}"))?;
        self.converter = Some(converter);
        self.buffer = Some(buffer);
        self.stream = Some(stream);
        Ok(())
    }

    fn push(&mut self, samples: &[i16]) -> Result<(), String> {
        let converted = self
            .converter
            .as_mut()
            .ok_or("Windows audio stream has not started")?
            .convert(samples)?;
        let mut buffer = self
            .buffer
            .as_ref()
            .ok_or("Windows device buffer is unavailable")?
            .lock()
            .map_err(|_| "Windows device buffer lock poisoned")?;
        if buffer.ended {
            return Err("Windows device buffer already ended".into());
        }
        buffer.samples.extend(converted)
    }

    fn finish(&mut self) -> Result<(), String> {
        self.buffer
            .as_ref()
            .ok_or("Windows device buffer is unavailable")?
            .lock()
            .map_err(|_| "Windows device buffer lock poisoned")?
            .ended = true;
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
            drop(stream);
        }
        self.buffer = None;
        self.converter = None;
    }

    fn poll(&mut self) -> Vec<BackendEvent> {
        if self.failed.swap(false, Ordering::AcqRel) {
            return vec![BackendEvent::Failed(
                "Windows audio device stream failed".into(),
            )];
        }
        let Some(buffer) = &self.buffer else {
            return Vec::new();
        };
        let Ok(mut buffer) = buffer.lock() else {
            return vec![BackendEvent::Failed(
                "Windows device buffer lock poisoned".into(),
            )];
        };
        let ended = buffer.ended;
        let empty = buffer.samples.is_empty();
        buffer.clock.poll(Instant::now(), ended, empty)
    }

    fn output_level(&mut self) -> f32 {
        if self.failed.load(Ordering::Acquire) {
            return 0.0;
        }
        let Some(buffer) = &self.buffer else {
            return 0.0;
        };
        let Ok(buffer) = buffer.try_lock() else {
            return 0.0;
        };
        buffer.meter.level(Instant::now())
    }
}

impl Drop for DeviceBackend {
    fn drop(&mut self) {
        self.stop();
    }
}
