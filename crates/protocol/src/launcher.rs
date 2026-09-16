//! Linux 网页启动器的本机服务管理契约，不进入 Windows 播放控制通道。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LauncherServiceId {
    Server,
    Tts,
    Windows,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LauncherServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
    External,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct LauncherService {
    pub id: LauncherServiceId,
    pub state: LauncherServiceState,
    pub managed: bool,
    pub message: String,
    pub url: String,
    pub log_path: String,
    pub can_start: bool,
    pub can_stop: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct LauncherSetup {
    pub configuration_path: String,
    pub server_config: String,
    pub llm_configured: bool,
    pub llm_message: String,
    pub windows_client_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct LauncherSnapshot {
    pub schema_version: u32,
    pub session_token: String,
    pub services: Vec<LauncherService>,
    pub setup: LauncherSetup,
}
