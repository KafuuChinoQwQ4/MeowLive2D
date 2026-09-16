//! Agent 用例的异步驱动与服务状态组装，模型 I/O 不进入业务锁。
mod mapping;
mod state;
mod worker;
pub use worker::run_agent;
