//! OBS 本地控制公开契约；仅包含状态、场景与录制，不暴露凭据或输出路径。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, PartialEq, Eq, TS)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObsOperation {
    Status,
    SetScene { scene_name: String },
    StartRecording,
    StopRecording,
}

impl<'de> Deserialize<'de> for ObsOperation {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Serde ignores extra fields on internally tagged unit variants, even with
        // deny_unknown_fields. Empty struct variants enforce the public boundary.
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
        enum WireOperation {
            Status {},
            SetScene { scene_name: String },
            StartRecording {},
            StopRecording {},
        }
        Ok(match WireOperation::deserialize(deserializer)? {
            WireOperation::Status {} => Self::Status,
            WireOperation::SetScene { scene_name } => Self::SetScene { scene_name },
            WireOperation::StartRecording {} => Self::StartRecording,
            WireOperation::StopRecording {} => Self::StopRecording,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct ObsSnapshot {
    pub connected: bool,
    pub recording: bool,
    pub current_scene: String,
    pub scenes: Vec<String>,
}
