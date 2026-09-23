//! 跨进程通信契约的唯一来源。与业务领域对象分离，按协议版本演进。
//!

pub const PROTOCOL_VERSION: u16 = 3;

pub mod agent;
pub mod agent_observability;
pub mod audio;
pub mod auth;
pub mod control;
pub mod event;
pub mod execution;
pub mod launcher;
pub mod live;
pub mod llm;
pub mod llm_runtime;
pub mod model_library;
pub mod obs;
pub mod resources;
pub mod training;
pub mod training_runtime;

pub mod viewers;

pub mod companionship;

pub mod memory;

pub mod relationships;

pub mod viewer_merge;
