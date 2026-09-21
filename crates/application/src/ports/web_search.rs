//! 网页检索的业务接口，模型只能提供查询文本，不能指定网络端点。
use std::{future::Future, pin::Pin};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub type SearchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<SearchResult>, String>> + Send + 'a>>;

pub trait WebSearch: Send + Sync {
    fn search(&self, query: String) -> SearchFuture<'_>;
}
