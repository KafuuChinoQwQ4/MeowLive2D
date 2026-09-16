//! 控制面板与桌面执行者共享的控制协议。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{audio::AudioFormat, execution::ExecutionReceipt};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct SpeechRequest {
    pub text: String,
    pub voice_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum SpeechStatus {
    Queued,
    Synthesizing,
    Ready,
    Playing,
    Completed,
    Cancelled,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct SpeechSnapshot {
    pub id: String,
    pub generation: u32,
    pub text: String,
    pub voice_id: String,
    pub status: SpeechStatus,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct ServerStatus {
    pub protocol_version: u16,
    pub session_id: String,
    pub bridge_connected: bool,
    pub generation: u32,
    pub speeches: Vec<SpeechSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerCommand {
    Hello {
        protocol_version: u16,
        session_id: String,
        bridge_id: String,
        generation: u32,
    },
    Speak {
        utterance_id: String,
        generation: u32,
        format: AudioFormat,
    },
    Stop {
        generation: u32,
    },
    Resource {
        request_id: String,
        operation: crate::resources::DesktopResourceOperation,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        protocol_version: u16,
    },
    Receipt {
        receipt: ExecutionReceipt,
    },
    ResourceResult {
        request_id: String,
        result: crate::resources::DesktopResourceResult,
    },
}
