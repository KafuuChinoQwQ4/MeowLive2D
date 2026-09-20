//! OBS 本地控制与设置契约；查询仅返回密码是否配置，不返回凭据或输出路径。
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct ObsSettingsSnapshot {
    pub enabled: bool,
    pub websocket_url: String,
    pub password_configured: bool,
    pub storage_available: bool,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct ObsSettingsRequest {
    pub enabled: bool,
    pub websocket_url: String,
    /// None retains the current credential; clearing is always explicit.
    pub password: Option<String>,
    pub clear_password: bool,
}

impl std::fmt::Debug for ObsSettingsRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObsSettingsRequest")
            .field("enabled", &self.enabled)
            .field("password_provided", &self.password.is_some())
            .field("clear_password", &self.clear_password)
            .finish_non_exhaustive()
    }
}
