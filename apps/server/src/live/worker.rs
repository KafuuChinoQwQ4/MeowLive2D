//! 连接、接收、清理和退避均在业务锁外等待；取消后不再接收旧连接事件。
use crate::state::AppState;
use meowlive_application::{
    agent::SubmitOutcome,
    ports::live_source::{LiveSource, LiveSourceError},
};
use meowlive_domain::event::LiveEvent;
use meowlive_protocol::live::LiveConnectionPhase as Phase;
use std::time::Duration;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub(super) async fn run(
    state: AppState,
    source: std::sync::Arc<dyn LiveSource>,
    cancel: CancellationToken,
    done: watch::Sender<bool>,
) {
    let mut delay = state.config.live.reconnect_initial_ms;
    let mut terminal_error = None;
    loop {
        if cancel.is_cancelled() {
            break;
        }
        // Do not drop an in-flight start: its result owns the remote session cleanup.
        let result = source.connect().await;
        let error = match result {
            Ok(mut connection) => {
                let ingestion_session = format!("live:{}", uuid::Uuid::new_v4());
                {
                    let mut inner = state.inner.lock().await;
                    if !cancel.is_cancelled() {
                        inner.live.snapshot.phase = Phase::Connected;
                        inner.live.snapshot.room_id = Some(connection.room_id().to_owned());
                        inner.live.snapshot.last_error = None;
                    }
                }
                let mut admitted = false;
                let failure = loop {
                    let next = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => break None,
                        result = connection.next() => result,
                    };
                    match next {
                        Ok(Some(event)) => {
                            admit(&state, &cancel, &ingestion_session, event).await;
                            admitted = true;
                        }
                        Ok(None) => break Some(LiveSourceError::new("直播事件连接已断开", true)),
                        Err(error) => break Some(error),
                    }
                };
                // Pause before slow cleanup; already queued speech still gets its receipt.
                if let Some(error) = &failure {
                    state.pause_agent().await;
                    let mut inner = state.inner.lock().await;
                    inner.live.snapshot.room_id = None;
                    if !cancel.is_cancelled() {
                        inner.live.snapshot.phase = if error.retryable {
                            Phase::Reconnecting
                        } else {
                            Phase::Disconnecting
                        };
                        inner.live.snapshot.last_error = Some(error.message.clone());
                    }
                }
                if let Err(error) = connection.close().await {
                    terminal_error = Some(format!("结束直播会话失败：{}", error.message));
                    break;
                }
                if admitted {
                    delay = state.config.live.reconnect_initial_ms;
                }
                failure
            }
            Err(error) => Some(error),
        };
        if cancel.is_cancelled() {
            break;
        }
        let Some(error) = error else {
            break;
        };
        state.pause_agent().await;
        if !error.retryable {
            terminal_error = Some(error.message);
            break;
        }
        {
            let mut inner = state.inner.lock().await;
            if cancel.is_cancelled() {
                break;
            }
            inner.live.snapshot.phase = Phase::Reconnecting;
            inner.live.snapshot.last_error = Some(error.message);
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(Duration::from_millis(delay)) => {},
        }
        delay = delay
            .saturating_mul(2)
            .min(state.config.live.reconnect_max_ms);
        let mut inner = state.inner.lock().await;
        if cancel.is_cancelled() {
            break;
        }
        inner.live.snapshot.reconnect_attempts =
            inner.live.snapshot.reconnect_attempts.saturating_add(1);
    }
    {
        let mut inner = state.inner.lock().await;
        inner.live.cancel = None;
        inner.live.snapshot.room_id = None;
        inner.live.snapshot.phase = if terminal_error.is_some() {
            Phase::Failed
        } else {
            Phase::Disconnected
        };
        inner.live.snapshot.last_error = terminal_error;
    }
    let _ = done.send(true);
}

async fn admit(state: &AppState, cancel: &CancellationToken, session: &str, mut event: LiveEvent) {
    if cancel.is_cancelled() {
        return;
    }
    let persistent = state.viewer_store.is_some() || state.config.viewers.enabled;
    if persistent {
        let result = match state.viewer_requests.try_acquire() {
            Ok(_permit) => {
                state
                    .persist_events(session, std::slice::from_ref(&event))
                    .await
            }
            Err(_) => {
                state
                    .viewer_gaps
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Err(crate::viewers::unavailable())
            }
        };
        let mut inner = state.inner.lock().await;
        match result {
            Ok(receipts) if receipts.first().is_some_and(|r| r.duplicate) => {
                inner.live.snapshot.duplicate_events =
                    inner.live.snapshot.duplicate_events.saturating_add(1);
                return;
            }
            Err(_) => {
                inner.live.snapshot.rejected_events =
                    inner.live.snapshot.rejected_events.saturating_add(1);
                inner.live.snapshot.last_error =
                    Some("观众数据库接收失败，已记录数据缺口；恢复后不会补播未知事件。".into());
                return;
            }
            _ => {}
        }
        let age = crate::viewers::utc_ms().saturating_sub(event.occurred_at_ms);
        if cancel.is_cancelled() || age >= state.config.agent.event_ttl_ms {
            inner.live.snapshot.last_error =
                Some("事件已持久保存，因取消或时效未进入回应队列。".into());
            return;
        }
    }
    let mut inner = state.inner.lock().await;
    if cancel.is_cancelled() {
        return;
    }
    let now = state.now_ms();
    let age_ms = if persistent {
        crate::viewers::utc_ms().saturating_sub(event.occurred_at_ms)
    } else {
        0
    };
    event.occurred_at_ms = now;
    match inner.agent.submit_with_age(event, now, age_ms) {
        Ok(SubmitOutcome::Accepted) => {
            inner.live.snapshot.accepted_events =
                inner.live.snapshot.accepted_events.saturating_add(1);
            state.agent_wake.notify_one();
        }
        Ok(SubmitOutcome::Duplicate) => {
            inner.live.snapshot.duplicate_events =
                inner.live.snapshot.duplicate_events.saturating_add(1);
        }
        Err(_) => {
            inner.live.snapshot.rejected_events =
                inner.live.snapshot.rejected_events.saturating_add(1);
            inner.live.snapshot.last_error = Some(
                if persistent {
                    "事件已持久保存，但待回应队列已满或事件无效，未安排回应。"
                } else {
                    "事件无效或待回应队列已满，已丢弃该事件。"
                }
                .into(),
            );
        }
    }
}
