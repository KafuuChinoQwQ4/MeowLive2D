//! Desktop WSL environment command and status contract.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentRequest {
    pub action: String,
    pub distro: Option<String>,
    pub model_id: Option<String>,
    pub config_path: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Distro {
    pub name: String,
    pub version: u8,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Backend {
    pub ready: bool,
    pub engine_root: String,
    pub python_path: String,
    #[serde(default)]
    pub model_root: String,
    pub gpu: bool,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub capability: String,
    pub downloaded: bool,
    pub selected: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSnapshot {
    pub phase: String,
    pub busy: bool,
    pub message: String,
    pub logs: Vec<String>,
    pub distros: Vec<Distro>,
    pub selected_distro: Option<String>,
    pub backend: Backend,
    pub models: Vec<Model>,
    pub progress: u8,
    pub inference_running: bool,
}
