//! 礼物优先，短窗口内同一观众礼物按有界原始事件组一起回应。
use super::{EventBatch, EventScheduler};
use crate::agent::EventStatus;
use meowlive_domain::event::{EventKind, LiveEvent};

impl EventScheduler {
    pub fn select(&mut self, now_ms: u64) -> EventBatch {
        self.expire(now_ms);
        let mut candidates: Vec<_> = self
            .records
            .iter()
            .filter(|row| row.status == EventStatus::Pending)
            .map(|row| row.event.clone())
            .collect();
        candidates.sort_by_key(|event| !matches!(event.kind, EventKind::Gift { .. }));
        let mut events = Vec::new();
        let mut groups = Vec::new();
        while !candidates.is_empty() && events.len() < self.limits.batch_size {
            let first = candidates.remove(0);
            let mut group = vec![first.id.clone()];
            let mut merged = vec![first.clone()];
            let mut index = 0;
            while index < candidates.len() && events.len() + merged.len() < self.limits.batch_size {
                if mergeable(&first, &candidates[index], self.limits.gift_merge_ms) {
                    let event = candidates.remove(index);
                    group.push(event.id.clone());
                    merged.push(event);
                } else {
                    index += 1;
                }
            }
            events.extend(merged);
            groups.push(group);
        }
        let ids: Vec<_> = events.iter().map(|event| event.id.clone()).collect();
        self.update(&ids, EventStatus::Deciding, None, None);
        EventBatch { events, groups }
    }
}

fn mergeable(first: &LiveEvent, other: &LiveEvent, window: u64) -> bool {
    window > 0
        && first.source == other.source
        && first.viewer == other.viewer
        && first.occurred_at_ms.abs_diff(other.occurred_at_ms) <= window
        && matches!((&first.kind, &other.kind), (EventKind::Gift { name: a, .. }, EventKind::Gift { name: b, .. }) if a == b)
}
