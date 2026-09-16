//! 由业务方定义的外部能力接口。实现位于 adapters 或应用入口的传输适配层。
//! 接口使用 domain 类型，不暴露 reqwest、SQLx、GPT-SoVITS 或跨端 DTO。

pub mod execution;
pub mod live_source;
pub mod llm;
pub mod speech;
pub mod storage;
pub mod training;
