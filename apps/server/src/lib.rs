//! 可注入适配器的主服务组装边界，供进程入口与集成测试复用。
pub mod agent;
pub mod bootstrap;
pub mod config;
mod gpu;
pub mod live;
pub mod llm_settings;
pub mod resources;
pub mod state;
pub mod transport;
pub mod worker;
