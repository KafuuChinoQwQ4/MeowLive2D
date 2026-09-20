//! 观众身份和直播事件的幂等接收与管理员查询端口。

use meowlive_domain::event::{EventKind, GiftMetadata, LiveEvent};
use std::{fmt, future::Future, pin::Pin};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreEventOutcome {
    pub duplicate: bool,
    pub viewer_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerIdentitySummary {
    pub platform: String,
    pub namespace: String,
    pub id_kind: String,
    pub external_id: String,
    pub last_confirmed_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerAliasSummary {
    pub alias: String,
    pub first_seen_at_ms: u64,
    pub last_seen_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerSummary {
    pub viewer_id: String,
    pub current_alias: Option<String>,
    pub alias_observed_at_ms: Option<u64>,
    /// At most 100 most recently observed aliases, newest first.
    pub aliases: Vec<ViewerAliasSummary>,
    /// At most 100 most recently confirmed identities, newest first.
    pub identities: Vec<ViewerIdentitySummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedEventSummary {
    pub event_id: String,
    pub source: String,
    pub session_id: String,
    pub viewer_id: Option<String>,
    pub viewer: String,
    pub occurred_at_ms: u64,
    pub received_at_ms: u64,
    pub kind: EventKind,
    pub gift_metadata: Option<GiftMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerStoreError {
    pub message: String,
}

impl ViewerStoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ViewerStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ViewerStoreError {}

pub type ViewerStoreFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ViewerStoreError>> + Send + 'a>>;

pub trait ViewerEventStore: Send + Sync {
    /// Persists a batch atomically. A duplicate source event has no profile or
    /// alias side effects and receives `duplicate = true` in its input order.
    fn accept_events<'a>(
        &'a self,
        scope_id: &'a str,
        session_id: &'a str,
        events: &'a [LiveEvent],
        received_at_ms: u64,
    ) -> ViewerStoreFuture<'a, Vec<StoreEventOutcome>>;

    fn list_viewers<'a>(
        &'a self,
        scope_id: &'a str,
        limit: u32,
        offset: u64,
    ) -> ViewerStoreFuture<'a, Vec<ViewerSummary>>;

    fn list_events<'a>(
        &'a self,
        scope_id: &'a str,
        limit: u32,
        offset: u64,
    ) -> ViewerStoreFuture<'a, Vec<PersistedEventSummary>>;
}
