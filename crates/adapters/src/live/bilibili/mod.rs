//! 哔哩哔哩直播开放平台的签名 HTTP、二进制长连接与事件标准化。

mod config;
mod connection;
mod http;
pub mod protocol;
pub mod signing;

pub use config::BilibiliConfig;
pub use connection::BilibiliLiveSource;
