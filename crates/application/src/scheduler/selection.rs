//! 礼物优先，短窗口内同一观众礼物按有界原始事件组一起回应。
use super::{EventBatch, EventScheduler};
use crate::agent::EventStatus;
use meowlive_domain::event::{EventKind, LiveEvent};

impl EventScheduler {
    pub fn select(&mut self, now_ms: u64, read_all: bool) -> EventBatch {
        self.expire(now_ms);
        let mut candidates: Vec<_> = self
            .records
            .iter()
            .filter(|row| row.status == EventStatus::Pending)
            .map(|row| row.event.clone())
            .collect();
        // Paid text has its own turn and cannot be removed by ordinary fairness narrowing.
        let targets = if let Some(sc) = candidates
            .iter()
            .filter(|e| matches!(e.kind, EventKind::SuperChat { .. }))
            .min_by_key(|e| e.occurred_at_ms)
            .cloned()
        {
            candidates = vec![sc];
            Vec::new()
        } else {
            let targets = self.prioritize(&mut candidates, now_ms);
            // Welcomes always have their own turn so the welcome cooldown
            // cannot be bypassed by choosing multiple entry events at once.
            if candidates
                .first()
                .is_some_and(|e| matches!(e.kind, EventKind::RoomEnter))
            {
                candidates.truncate(1);
            } else if read_all {
                match candidates.first().map(|e| &e.kind) {
                    Some(EventKind::Chat { .. }) => candidates.truncate(1),
                    Some(EventKind::Gift { .. }) => {
                        candidates.retain(|e| matches!(e.kind, EventKind::Gift { .. }))
                    }
                    _ => {}
                }
            }
            targets
        };
        let mut events = Vec::new();
        let mut groups = Vec::new();
        let mut read_chars = 0;
        while !candidates.is_empty() && events.len() < self.limits.batch_size {
            let first = candidates.remove(0);
            let chars = crate::agent::reading_prefix(&first).chars().count();
            // Every selected original must fit completely together with a 500-char model reply.
            if read_chars + chars > 700 {
                continue;
            }
            read_chars += chars;
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
        EventBatch {
            events,
            groups,
            targets,
        }
    }
}

fn mergeable(first: &LiveEvent, other: &LiveEvent, window: u64) -> bool {
    window > 0
        && first.source == other.source
        && first.viewer_identity.is_some()
        && first.viewer_identity == other.viewer_identity
        && first.occurred_at_ms.abs_diff(other.occurred_at_ms) <= window
        && matches!((&first.kind, &other.kind), (EventKind::Gift { name: a, .. }, EventKind::Gift { name: b, .. }) if a == b)
}
