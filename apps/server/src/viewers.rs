//! 持久事件接收先于即时调度；数据库等待不持有 Agent/播放锁。
use crate::{state::AppState, transport::error::ApiError};
use axum::http::StatusCode;
use meowlive_application::{agent::SubmitOutcome, ports::viewers::StoreEventOutcome};
use meowlive_domain::event::LiveEvent;
use meowlive_protocol::agent::EventBatchResult;
use std::sync::atomic::Ordering;

pub fn utc_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as u64
}

impl AppState {
    pub(crate) async fn persist_events(
        &self,
        session_id: &str,
        events: &[LiveEvent],
    ) -> Result<Vec<StoreEventOutcome>, ApiError> {
        let Some(store) = &self.viewer_store else {
            return Err(unavailable());
        };
        let result = store
            .accept_events(&self.config.viewers.scope_id, session_id, events, utc_ms())
            .await;
        result.map_err(|_| {
            self.viewer_gaps
                .fetch_add(events.len() as u64, Ordering::Relaxed);
            unavailable()
        })
    }

    pub(crate) async fn submit_persisted_events(
        &self,
        mut events: Vec<LiveEvent>,
    ) -> Result<EventBatchResult, ApiError> {
        let _permit = self.viewer_requests.try_acquire().map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "viewer_ingest_busy",
                "事件接收繁忙，请稍后重试",
            )
        })?;
        // This HTTP endpoint is an administrator simulator/replay input, not a trusted platform.
        for event in &mut events {
            event.source = "simulator".into();
            if let Some(identity) = &mut event.viewer_identity {
                identity.namespace = "simulator".into();
            }
        }
        let receipts = self
            .persist_events(&format!("simulator:{}", self.session_id), &events)
            .await?;
        let mut result = EventBatchResult {
            accepted: 0,
            duplicates: 0,
            persisted: Some(0),
            unscheduled: Some(0),
        };
        let mut inner = self.inner.lock().await;
        let now = self.now_ms();
        for (mut event, receipt) in events.into_iter().zip(receipts) {
            if receipt.duplicate {
                result.duplicates += 1;
                continue;
            }
            *result.persisted.as_mut().unwrap() += 1;
            let age_ms = utc_ms().saturating_sub(event.occurred_at_ms);
            if age_ms >= self.config.agent.event_ttl_ms {
                *result.unscheduled.as_mut().unwrap() += 1;
                continue;
            }
            // Only the scheduler copy uses process-relative time.
            event.occurred_at_ms = now;
            match inner.agent.submit_with_age(event, now, age_ms) {
                Ok(SubmitOutcome::Accepted) => result.accepted += 1,
                _ => *result.unscheduled.as_mut().unwrap() += 1,
            }
        }
        self.agent_wake.notify_one();
        Ok(result)
    }
}

pub(crate) fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "viewer_store_unavailable",
        "观众事件持久化暂不可用；该批次未确认接收，直播数据可能存在缺口",
    )
}
