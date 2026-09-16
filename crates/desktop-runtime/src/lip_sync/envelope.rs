//! 根据实际流逝时间平滑输出，避免网络分片大小或轮询次数改变口型速度。
use super::LipSyncConfig;
use std::time::Duration;

pub struct LipSync {
    config: LipSyncConfig,
    value: f64,
}

impl LipSync {
    pub fn new(config: LipSyncConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config, value: 0.0 })
    }

    pub fn sample(&mut self, rms: f32, elapsed: Duration) -> f64 {
        if !rms.is_finite() || rms < 0.0 {
            return self.reset();
        }
        let target = ((f64::from(rms).clamp(0.0, 1.0) - self.config.noise_floor).max(0.0)
            * self.config.gain)
            .min(1.0);
        let millis = if target > self.value {
            self.config.attack_ms
        } else {
            self.config.release_ms
        };
        let factor = 1.0 - (-elapsed.as_secs_f64() / (millis as f64 / 1000.0)).exp();
        self.value += (target - self.value) * factor;
        if self.value < 0.001 && target == 0.0 {
            self.value = 0.0;
        }
        self.value.clamp(0.0, 1.0)
    }

    pub fn reset(&mut self) -> f64 {
        self.value = 0.0;
        0.0
    }
}
