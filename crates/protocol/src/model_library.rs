//! 本机环境检查和模型资料库契约；与 Windows 播放协议独立。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ModelEnvironment {
    pub kind: String,
    pub release: String,
    pub distro: String,
    pub ready: bool,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ModelRuntime {
    pub engine_root: String,
    pub python_path: String,
    pub ready: bool,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct CatalogModel {
    pub id: String,
    pub purpose: String,
    pub name: String,
    pub languages: String,
    pub description: String,
    pub license: String,
    pub homepage: String,
    pub source_url: String,
    pub compatibility: String,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct InstalledModel {
    pub id: String,
    pub purpose: String,
    pub model_id: String,
    pub name: String,
    pub path: String,
    pub ready: bool,
    pub selected: bool,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ModelDownload {
    pub id: String,
    pub model_id: String,
    pub state: String,
    pub message: String,
    pub downloaded_bytes: f64,
    pub total_bytes: f64,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ModelLibrarySnapshot {
    pub schema_version: u32,
    pub environment: ModelEnvironment,
    pub runtime: ModelRuntime,
    pub scan_roots: Vec<String>,
    pub installed: Vec<InstalledModel>,
    pub catalog: Vec<CatalogModel>,
    pub downloads: Vec<ModelDownload>,
    pub selected_id: Option<String>,
    pub asr_selected_id: Option<String>,
    pub asr_error: Option<String>,
}
