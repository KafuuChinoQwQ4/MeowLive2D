//! Desktop App updater state, shared with the frontend.
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, TS)]
pub struct UpdateStatus {
    pub phase: String,
    pub current_tag: String,
    pub available_tag: Option<String>,
    pub release_url: Option<String>,
    pub release_notes: Option<String>,
    pub message: String,
    #[ts(type = "number")]
    pub downloaded_bytes: u64,
    #[ts(type = "number")]
    pub reused_bytes: u64,
    #[ts(type = "number")]
    pub total_bytes: u64,
}
