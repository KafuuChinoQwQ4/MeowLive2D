//! 设备电平到口型的纯规则与音频装饰器，网络连接由 avatar 独立处理。

mod backend;
mod config;
mod envelope;

pub use backend::LipSyncBackend;
pub use config::LipSyncConfig;
pub use envelope::LipSync;
