//! 主服务与 Windows 的指令、回执及重连状态核对消息。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Started,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct ExecutionReceipt {
    pub utterance_id: String,
    pub generation: u32,
    pub status: ExecutionStatus,
    pub error: Option<String>,
}
