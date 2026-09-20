//! 独立后台模型能力；配置与密钥由外部组合根注入。
pub use meowlive_domain::memory::{
    Evidence, MemoryCandidate, MemoryKind, MemorySource, MemoryStatus,
};
use std::{future::Future, pin::Pin};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryError {
    pub message: String,
    pub retryable: bool,
}
impl MemoryError {
    pub fn new(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            message: message.into(),
            retryable,
        }
    }
}
impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for MemoryError {}
pub type MemoryFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, MemoryError>> + Send + 'a>>;
pub trait MemoryExtractor: Send + Sync {
    fn extract<'a>(&'a self, sources: &'a [MemorySource])
    -> MemoryFuture<'a, Vec<MemoryCandidate>>;
}
pub trait MemoryEmbedder: Send + Sync {
    fn embed<'a>(&'a self, texts: &'a [String]) -> MemoryFuture<'a, EmbeddingBatch>;
}
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddingBatch {
    pub model: String,
    pub dimensions: usize,
    pub vectors: Vec<Vec<f32>>,
}
