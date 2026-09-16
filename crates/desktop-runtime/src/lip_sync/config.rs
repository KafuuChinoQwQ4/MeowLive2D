//! 口型增益、噪声门限、平滑与发送频率的可校验配置。
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LipSyncConfig {
    pub gain: f64,
    pub noise_floor: f64,
    pub attack_ms: u64,
    pub release_ms: u64,
    pub update_hz: u32,
}

impl Default for LipSyncConfig {
    fn default() -> Self {
        Self {
            gain: 4.0,
            noise_floor: 0.01,
            attack_ms: 30,
            release_ms: 100,
            update_hz: 30,
        }
    }
}

impl LipSyncConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.gain.is_finite()
            || !(0.1..=20.0).contains(&self.gain)
            || !self.noise_floor.is_finite()
            || !(0.0..=0.5).contains(&self.noise_floor)
            || !(1..=1000).contains(&self.attack_ms)
            || !(1..=2000).contains(&self.release_ms)
            || !(10..=30).contains(&self.update_hz)
        {
            return Err("口型配置无效：gain 0.1..20、noise_floor 0..0.5、attack_ms 1..1000、release_ms 1..2000、update_hz 10..30".into());
        }
        Ok(())
    }
}
