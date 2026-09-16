//! 独立 VTube Studio 连接、授权和最新口型参数注入。
mod client;
mod config;
pub mod resources;
pub(crate) mod switching;
mod token;
mod worker;

pub use config::VtsConfig;
pub use resources::{ResourceError, VtsHotkeyInfo, VtsModelInfo, VtsResources};
pub use worker::run;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AvatarStatus {
    Disabled,
    Connecting,
    AwaitingAuthorization,
    Connected,
    Disconnected,
    AuthorizationRequired,
    Failed(String),
}
