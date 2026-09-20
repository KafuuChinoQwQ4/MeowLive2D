//! 完成回执先由桥接写本地持久暂存，再在业务锁外幂等提交数据库。
use crate::state::AppState;
use std::{collections::VecDeque, sync::atomic::Ordering, time::Duration};

pub async fn run_receipts(state: AppState) {
    let capacity = 512usize.max(state.config.agent.history_limit.saturating_mul(2));
    let mut fallback = VecDeque::<(String, u64)>::new();
    let mut shutdown_deadline = None;
    loop {
        // Production completions were journaled before recognition. The fallback
        // is used only by explicitly assembled in-memory application harnesses.
        if state.receipt_journal.is_some()
            || fallback
                .len()
                .saturating_add(state.config.agent.history_limit)
                <= capacity
        {
            let mut inner = state.inner.lock().await;
            state.sync_agent(&mut inner);
            for completion in inner.agent.take_completed() {
                if state.receipt_journal.is_none() && state.companionship_store.is_some() {
                    fallback.push_back((completion.speech_id, crate::viewers::utc_ms()));
                }
            }
        }
        let records = if let Some(journal) = state.receipt_journal.clone() {
            let scope = state.config.viewers.scope_id.clone();
            match tokio::task::spawn_blocking(move || journal.pending(&scope, 4096)).await {
                Ok(Ok(records)) => records,
                _ => {
                    state.receipt_failures.fetch_add(1, Ordering::Relaxed);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        let count = if state.receipt_journal.is_some() {
            records.len()
        } else {
            fallback.len()
        };
        state
            .receipts_pending
            .store(count as u32, Ordering::Release);
        *state.receipt_problems.lock().await = records
            .iter()
            .filter(|r| r.attempts > 0)
            .take(20)
            .map(|r| meowlive_protocol::companionship::ReceiptProblem {
                speech_id: r.speech_id.clone(),
                attempts: r.attempts,
            })
            .collect();
        let next = records
            .first()
            .map(|r| (r.speech_id.clone(), r.completed_at_ms))
            .or_else(|| fallback.pop_front());
        let mut delay = Duration::from_millis(100);
        if let (Some(store), Some((id, time))) = (&state.companionship_store, next) {
            let success = store
                .complete_reply(&state.config.viewers.scope_id, &id, time)
                .await
                .is_ok();
            if !success {
                state.receipt_failures.fetch_add(1, Ordering::Relaxed);
                delay = Duration::from_secs(1);
            }
            if let Some(journal) = state.receipt_journal.clone() {
                let scope = state.config.viewers.scope_id.clone();
                let result = tokio::task::spawn_blocking(move || {
                    if success {
                        journal.acknowledge(&scope, &id)
                    } else {
                        journal.failed(&scope, &id)
                    }
                })
                .await;
                if !matches!(result, Ok(Ok(()))) {
                    state.receipt_failures.fetch_add(1, Ordering::Relaxed);
                }
            } else if !success {
                fallback.push_back((id, time));
            }
        }
        if state.stopping.is_cancelled() {
            let deadline = shutdown_deadline
                .get_or_insert_with(|| tokio::time::Instant::now() + Duration::from_secs(3));
            if count == 0 || tokio::time::Instant::now() >= *deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        } else {
            tokio::select! {_=state.stopping.cancelled()=>{},_=tokio::time::sleep(delay)=>{}}
        }
    }
}
