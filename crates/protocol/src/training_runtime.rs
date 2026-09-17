//! 受管 TTS 服务与语音模型内存生命周期分离的公开契约。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct TrainingModelRuntimeSnapshot {
    pub supported: bool,
    pub state: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct TrainingModelRuntimeRequest {
    pub enabled: bool,
}
