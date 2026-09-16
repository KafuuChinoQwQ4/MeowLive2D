//! 固定容量的设备播放能量时间线；采集与查询均不分配内存。

use std::time::{Duration, Instant};

#[derive(Default)]
pub struct SampleEnergy {
    sum_squares: f64,
    count: usize,
}

impl SampleEnergy {
    pub fn add(&mut self, sample: f32) {
        let sample = if sample.is_finite() {
            f64::from(sample.clamp(-1.0, 1.0))
        } else {
            0.0
        };
        self.sum_squares += sample * sample;
        self.count += 1;
    }

    pub fn rms(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        (self.sum_squares / self.count as f64).sqrt() as f32
    }
}

const CAPACITY: usize = 256;

#[derive(Clone, Copy)]
struct Interval {
    start: Instant,
    end: Instant,
    rms: f32,
}

pub struct OutputMeter {
    intervals: [Option<Interval>; CAPACITY],
    next: usize,
}

impl Default for OutputMeter {
    fn default() -> Self {
        Self {
            intervals: [None; CAPACITY],
            next: 0,
        }
    }
}

impl OutputMeter {
    /// Records one output interval; overwrites the oldest observation at capacity.
    pub fn record(
        &mut self,
        now: Instant,
        latency: Duration,
        frames: usize,
        sample_rate: u32,
        energy: SampleEnergy,
    ) {
        if frames == 0 || sample_rate == 0 {
            return;
        }
        let start = now + latency;
        let end = start + Duration::from_secs_f64(frames as f64 / f64::from(sample_rate));
        self.intervals[self.next] = Some(Interval {
            start,
            end,
            rms: energy.rms(),
        });
        self.next = (self.next + 1) % CAPACITY;
    }

    /// Future samples and expired intervals never contribute to the current level.
    pub fn level(&self, now: Instant) -> f32 {
        for offset in 0..CAPACITY {
            let index = (self.next + CAPACITY - 1 - offset) % CAPACITY;
            if let Some(interval) = self.intervals[index] {
                if interval.start <= now && now < interval.end {
                    return interval.rms;
                }
            }
        }
        0.0
    }

    pub fn clear(&mut self) {
        self.intervals.fill(None);
        self.next = 0;
    }
}
