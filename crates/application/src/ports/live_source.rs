//! 统一直播事件输入接口，支持真实平台、模拟和回放来源。
use meowlive_domain::event::LiveEvent;
use std::{fmt, future::Future, pin::Pin};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveSourceError {
    pub message: String,
    pub retryable: bool,
}

impl LiveSourceError {
    pub fn new(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            message: message.into(),
            retryable,
        }
    }
}
impl fmt::Display for LiveSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for LiveSourceError {}

pub type LiveFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, LiveSourceError>> + Send + 'a>>;

pub trait LiveSource: Send + Sync {
    /// Callers let this bounded operation finish, then close on cancellation,
    /// so a newly created remote session always has a cleanup owner.
    fn connect(&self) -> LiveFuture<'_, Box<dyn LiveConnection>>;
}

pub trait LiveConnection: Send {
    fn room_id(&self) -> &str;
    /// Drives heartbeats while waiting. Dropping this future must leave close usable.
    fn next(&mut self) -> LiveFuture<'_, Option<LiveEvent>>;
    /// Explicit, bounded cleanup of the remote session; safe to call more than once.
    fn close(&mut self) -> LiveFuture<'_, ()>;
}
