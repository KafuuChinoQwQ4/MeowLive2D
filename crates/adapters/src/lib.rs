//! 外部能力实现：依赖业务层定义的 ports，不反向定义业务规则。
//! 服务地址、认证、超时等由应用入口注入；供应商差异不越过此边界。

pub mod live;
pub mod llm;
pub mod runtime;
pub mod speech;
pub mod storage;
pub mod training;
