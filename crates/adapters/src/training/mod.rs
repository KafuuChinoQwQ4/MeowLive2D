//! 独立训练进程适配，处理进程状态与训练产物；调度策略属于 application。
mod store;
#[cfg(test)]
mod tests;
pub use store::FileTrainingStore;
mod process;
pub use process::{ProcessTrainingConfig, ProcessTrainingEngine};
