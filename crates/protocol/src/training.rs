//! 微调任务与成对权重版本公开契约；路径及训练日志保留在服务端。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct TrainingJob {
    pub id: String,
    pub name: String,
    pub voice_id: String,
    pub status: String,
    pub progress: u8,
    pub message: String,
    pub clip_count: u32,
    #[ts(type = "number")]
    pub created_at_ms: u64,
    #[ts(type = "number")]
    pub updated_at_ms: u64,
    pub version_id: Option<String>,
    #[serde(default)]
    pub performance: TrainingPerformance,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct ModelVersion {
    pub id: String,
    pub job_id: String,
    pub voice_id: String,
    pub name: String,
    pub engine: String,
    pub model_version: String,
    pub auditioned: bool,
    pub saved: bool,
    pub active: bool,
    pub available: bool,
    #[ts(type = "number")]
    pub created_at_ms: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct TrainingSnapshot {
    pub enabled: bool,
    pub busy: bool,
    pub jobs: Vec<TrainingJob>,
    pub versions: Vec<ModelVersion>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct TrainingClipMetadata {
    #[serde(default)]
    pub text: String,
    pub language: String,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum TrainingTextMode {
    AudioOnly,
    #[default]
    ReviewedText,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct TrainingCreateRequest {
    pub name: String,
    pub voice_id: String,
    pub sovits_epochs: u16,
    pub gpt_epochs: u16,
    pub reviewed: bool,
    #[serde(default)]
    pub text_mode: TrainingTextMode,
    #[serde(default)]
    pub performance: TrainingPerformance,
    pub clips: Vec<TrainingClipMetadata>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct TrainingTranscribeRequest {
    pub language: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct TrainingTranscription {
    pub text: String,
    pub language: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct TrainingAuditionRequest {
    pub version_id: String,
    pub text: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct RuntimeMeasurement {
    #[ts(type = "number")]
    pub measured_at_ms: u64,
    #[ts(type = "number")]
    pub llm_ms: u64,
    #[ts(type = "number")]
    pub tts_ms: u64,
    #[ts(type = "number")]
    pub total_ms: u64,
    pub gpu_name: String,
    pub gpu_total_mib: u32,
    pub gpu_peak_used_mib: u32,
    pub voice_id: String,
    pub reply: String,
    pub passed: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct RuntimePresetSnapshot {
    pub mode: String,
    pub model: String,
    pub local_only: bool,
    pub verified: bool,
    pub max_tokens: u32,
    pub timeout_seconds: u32,
    pub measurement: Option<RuntimeMeasurement>,
    pub message: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMeasureRequest {
    pub voice_id: String,
    pub text: String,
}
