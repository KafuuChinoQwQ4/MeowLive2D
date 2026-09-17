//! 训练状态、受限数据集与音色绑定的成对模型版本。

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_holds_busy_until_terminal_state() {
        assert!(!TrainingState::Cancelling.is_terminal());
        assert!(TrainingState::Cancelled.is_terminal());
        assert!(TrainingState::Interrupted.is_terminal());
    }
    #[test]
    fn parameters_reject_paths_and_unbounded_epochs() {
        let mut parameters = TrainingParameters {
            name: "测试".into(),
            voice_id: "default".into(),
            sovits_epochs: 1,
            gpt_epochs: 1,
            performance: TrainingPerformance::default(),
        };
        assert!(parameters.validate().is_ok());
        parameters.voice_id = "../voice".into();
        assert!(parameters.validate().is_err());
        parameters.voice_id = "default".into();
        parameters.gpt_epochs = 21;
        assert!(parameters.validate().is_err());
    }
    #[test]
    fn performance_defaults_are_conservative_and_bounds_are_enforced() {
        let defaults = TrainingPerformance::default();
        assert_eq!(
            defaults,
            TrainingPerformance {
                batch_size: 1,
                data_workers: 1,
                cpu_threads: 2,
                gpu_index: 0,
                low_memory: true,
            }
        );
        assert!(defaults.validate().is_ok());
        for invalid in [
            TrainingPerformance {
                batch_size: 0,
                ..defaults
            },
            TrainingPerformance {
                batch_size: 17,
                ..defaults
            },
            TrainingPerformance {
                data_workers: 9,
                ..defaults
            },
            TrainingPerformance {
                cpu_threads: 0,
                ..defaults
            },
            TrainingPerformance {
                cpu_threads: 17,
                ..defaults
            },
            TrainingPerformance {
                gpu_index: 16,
                ..defaults
            },
        ] {
            assert!(invalid.validate().is_err(), "accepted {invalid:?}");
        }
        assert!(
            TrainingPerformance {
                data_workers: 0,
                ..defaults
            }
            .validate()
            .is_ok()
        );
    }
    #[test]
    fn artifacts_must_be_the_fixed_pair_inside_job() {
        assert!(ArtifactPair::v2().validate().is_ok());
        let mut pair = ArtifactPair::v2();
        pair.gpt = "../outside.ckpt".into();
        assert!(pair.validate().is_err());
    }
}

pub const MAX_TRAINING_JOBS: usize = 64;
pub const MAX_TRAINING_CLIPS: usize = 32;
pub const MAX_CLIP_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_DATASET_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrainingState {
    Queued,
    Preparing,
    Training,
    Validating,
    Succeeded,
    Failed,
    Cancelling,
    Cancelled,
    Interrupted,
}
impl TrainingState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Preparing => "preparing",
            Self::Training => "training",
            Self::Validating => "validating",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelling => "cancelling",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "queued" => Self::Queued,
            "preparing" => Self::Preparing,
            "training" => Self::Training,
            "validating" => Self::Validating,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelling" => Self::Cancelling,
            "cancelled" => Self::Cancelled,
            "interrupted" => Self::Interrupted,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrainingPerformance {
    pub batch_size: u16,
    pub data_workers: u16,
    pub cpu_threads: u16,
    pub gpu_index: u16,
    pub low_memory: bool,
}
impl Default for TrainingPerformance {
    fn default() -> Self {
        Self {
            batch_size: 1,
            data_workers: 1,
            cpu_threads: 2,
            gpu_index: 0,
            low_memory: true,
        }
    }
}
impl TrainingPerformance {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if !(1..=16).contains(&self.batch_size)
            || self.data_workers > 8
            || !(1..=16).contains(&self.cpu_threads)
            || self.gpu_index > 15
        {
            return Err(TrainingError::Invalid("训练性能参数无效".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrainingParameters {
    pub name: String,
    pub voice_id: String,
    pub sovits_epochs: u16,
    pub gpt_epochs: u16,
    pub performance: TrainingPerformance,
}
impl TrainingParameters {
    pub fn validate(&self) -> Result<(), TrainingError> {
        self.performance.validate()?;
        if !crate::resources::valid_name(&self.name)
            || crate::voice::VoiceId::new(&self.voice_id).is_err()
            || !(1..=20).contains(&self.sovits_epochs)
            || !(1..=20).contains(&self.gpt_epochs)
        {
            return Err(TrainingError::Invalid("训练名称、音色或轮次无效".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrainingClip {
    pub text: String,
    pub language: String,
    pub wav: Vec<u8>,
}
impl TrainingClip {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.wav.is_empty()
            || self.wav.len() > MAX_CLIP_BYTES
            || (!self.text.is_empty() && self.text.trim().is_empty())
            || self.text.chars().count() > 500
            || self.text.chars().any(char::is_control)
            || self.text.contains('|')
            || !matches!(self.language.as_str(), "zh" | "en" | "ja" | "ko" | "yue")
        {
            return Err(TrainingError::Invalid(
                "片段需要有效的 WAV、明确语言及有效文本，自动识别时文本须留空".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPair {
    pub model_version: String,
    pub gpt: String,
    pub sovits: String,
}
impl ArtifactPair {
    pub fn v2() -> Self {
        Self {
            model_version: "v2".into(),
            gpt: "artifacts/gpt.ckpt".into(),
            sovits: "artifacts/sovits.pth".into(),
        }
    }
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self != &Self::v2() {
            return Err(TrainingError::Invalid(
                "训练产物必须是任务内成对的 v2 模型".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrainingJob {
    pub id: String,
    pub parameters: TrainingParameters,
    pub state: TrainingState,
    pub clip_count: usize,
    pub total_bytes: usize,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub progress: u8,
    pub message: String,
    pub artifacts: Option<ArtifactPair>,
    pub auditioned: bool,
    pub saved: bool,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TrainingCatalog {
    pub jobs: Vec<TrainingJob>,
    pub active_versions: std::collections::BTreeMap<String, String>,
}
impl TrainingCatalog {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.jobs.len() > MAX_TRAINING_JOBS || self.active_versions.len() > MAX_TRAINING_JOBS {
            return Err(TrainingError::Capacity);
        }
        let mut ids = std::collections::HashSet::new();
        for job in &self.jobs {
            job.parameters.validate()?;
            if !valid_job_id(&job.id)
                || !ids.insert(&job.id)
                || !(2..=MAX_TRAINING_CLIPS).contains(&job.clip_count)
                || job.total_bytes > MAX_DATASET_BYTES
                || job.progress > 100
                || job.message.chars().count() > 200
                || job.artifacts.is_some() != (job.state == TrainingState::Succeeded)
                || (job.auditioned && job.state != TrainingState::Succeeded)
                || (job.saved && !job.auditioned)
            {
                return Err(TrainingError::Store("训练历史格式无效".into()));
            }
            if let Some(pair) = &job.artifacts {
                pair.validate()?;
            }
        }
        if self
            .jobs
            .iter()
            .filter(|job| !job.state.is_terminal())
            .count()
            > 1
        {
            return Err(TrainingError::Busy);
        }
        for (voice, id) in &self.active_versions {
            if !self.jobs.iter().any(|job| {
                &job.id == id
                    && &job.parameters.voice_id == voice
                    && job.auditioned
                    && job.saved
                    && job.state == TrainingState::Succeeded
            }) {
                return Err(TrainingError::Store("激活模型引用无效".into()));
            }
        }
        Ok(())
    }
}
pub fn valid_job_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase(),
        })
        && value.as_bytes()[14] == b'4'
        && matches!(value.as_bytes()[19], b'8' | b'9' | b'a' | b'b')
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrainingError {
    Disabled,
    Busy,
    NotFound,
    Capacity,
    Invalid(String),
    Store(String),
    Engine(String),
    Cancelled,
    NotAuditioned,
}
impl std::fmt::Display for TrainingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Disabled => "训练服务尚未配置",
            Self::Busy => "训练或模型操作正在进行，请稍后重试",
            Self::NotFound => "训练任务不存在",
            Self::Capacity => "训练历史已达到上限",
            Self::Invalid(message) | Self::Store(message) | Self::Engine(message) => message,
            Self::Cancelled => "训练已取消",
            Self::NotAuditioned => "请先成功试听这个模型版本",
        })
    }
}
impl std::error::Error for TrainingError {}
