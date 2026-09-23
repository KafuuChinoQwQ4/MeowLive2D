//! HTTP / WebSocket 输入与输出适配；在协议 DTO 与领域对象之间进行映射。
//! 跨端执行适配在此实现 application::ports::execution，避免业务层依赖协议。

pub mod agent;
pub mod agent_observability;
pub mod auth;
pub mod bridge;
pub mod error;
pub mod http;
pub mod live;
pub mod llm;
pub mod llm_runtime;
pub mod mapping;
pub mod obs;
pub mod origin;
pub mod resources;
pub mod runtime;
pub mod training;
pub mod training_models;
pub mod websocket;

pub mod viewers;

pub mod companionship;

pub mod memory;

mod relationships;

mod viewer_merge;
