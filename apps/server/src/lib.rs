//! 可注入适配器的主服务组装边界，供进程入口与集成测试复用。
pub mod agent;
pub mod agent_settings;
pub mod auth;
pub mod bootstrap;
pub mod config;
mod gpu;
pub mod live;
pub mod live_settings;
pub mod llm_settings;
pub mod resources;
pub mod state;
pub mod transport;
pub mod worker;

pub mod viewers;

pub mod companionship;

pub mod memory;

pub mod graph;

#[cfg(test)]
mod memory_worker_tests;
