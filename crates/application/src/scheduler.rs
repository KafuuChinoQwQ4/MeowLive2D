//! 有界事件调度：待选状态与终态展示历史独立，去重不依赖展示历史。
mod selection;
use crate::agent::{AgentLimits, EventRecord, EventStatus, SubmitOutcome};
use meowlive_domain::event::LiveEvent;
use std::collections::{HashSet, VecDeque};

#[derive(Clone)]
pub(crate) struct EventScheduler {
    pub records: VecDeque<EventRecord>,
    pub limits: AgentLimits,
    seen: HashSet<String>,
    seen_order: VecDeque<String>,
}

#[derive(Clone)]
pub(crate) struct EventBatch {
    pub events: Vec<LiveEvent>,
    pub groups: Vec<Vec<String>>,
}

impl EventScheduler {
    pub fn new(limits: AgentLimits) -> Self {
        Self {
            records: VecDeque::new(),
            limits,
            seen: HashSet::new(),
            seen_order: VecDeque::new(),
        }
    }

    pub fn submit(&mut self, event: LiveEvent, now_ms: u64) -> Result<SubmitOutcome, String> {
        event.validate()?;
        if event.occurred_at_ms > now_ms.saturating_add(5000) {
            return Err("event timestamp is more than 5000 milliseconds in the future".into());
        }
        self.expire(now_ms);
        if self.seen.contains(&event.id)
            || self
                .records
                .iter()
                .any(|row| row.event.id == event.id && !row.status.is_terminal())
        {
            return Ok(SubmitOutcome::Duplicate);
        }
        let expired = now_ms.saturating_sub(event.occurred_at_ms) >= self.limits.event_ttl_ms;
        if !expired
            && self
                .records
                .iter()
                .filter(|row| row.status == EventStatus::Pending)
                .count()
                >= self.limits.pending_capacity
        {
            return Err("pending event capacity exceeded".into());
        }
        // An identity may be reused after bounded dedup eviction; remove its old terminal row.
        self.records.retain(|row| row.event.id != event.id);
        self.seen.insert(event.id.clone());
        self.seen_order.push_back(event.id.clone());
        while self.seen_order.len() > self.limits.dedup_capacity {
            if let Some(id) = self.seen_order.pop_front() {
                self.seen.remove(&id);
            }
        }
        self.records.push_back(EventRecord {
            event,
            status: if expired {
                EventStatus::Expired
            } else {
                EventStatus::Pending
            },
            speech_id: None,
            error: None,
        });
        self.prune();
        Ok(SubmitOutcome::Accepted)
    }

    pub fn expire(&mut self, now_ms: u64) {
        for row in &mut self.records {
            if row.status == EventStatus::Pending
                && now_ms.saturating_sub(row.event.occurred_at_ms) >= self.limits.event_ttl_ms
            {
                row.status = EventStatus::Expired;
            }
        }
        self.prune();
    }

    pub fn update(
        &mut self,
        ids: &[String],
        status: EventStatus,
        speech_id: Option<&str>,
        error: Option<&str>,
    ) {
        for row in &mut self.records {
            if ids.contains(&row.event.id) && !row.status.is_terminal() {
                row.status = status;
                row.speech_id = speech_id.map(str::to_owned);
                row.error = error.map(|message| message.chars().take(1000).collect());
            }
        }
        self.prune();
    }

    pub fn cancel_pending(&mut self) {
        for row in &mut self.records {
            if row.status == EventStatus::Pending {
                row.status = EventStatus::Cancelled;
            }
        }
        self.prune();
    }

    pub fn prune(&mut self) {
        let mut excess = self
            .records
            .iter()
            .filter(|row| row.status.is_terminal())
            .count()
            .saturating_sub(self.limits.history_limit);
        self.records.retain(|row| {
            if excess > 0 && row.status.is_terminal() {
                excess -= 1;
                false
            } else {
                true
            }
        });
    }
}
