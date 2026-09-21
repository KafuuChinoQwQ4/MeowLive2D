//! 模型协议适配。兼容同一协议的云端与本地服务复用实现，其他协议独立添加。

mod config;
mod prompt;
mod response;
mod runtime;

pub mod models;
pub mod multi_provider;
pub mod openai_compatible;
pub mod reasoning;
