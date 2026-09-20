//! 完成回执驱动的观众公平、有限重选与保守追问接续。
use super::EventScheduler;
use crate::agent::EventStatus;
use meowlive_domain::event::{EventKind, LiveEvent};
use std::collections::HashSet;

fn key(event: &LiveEvent) -> Option<String> {
    event.viewer_identity.as_ref().map(|id| {
        format!(
            "{}:{}{}:{}{:?}:{}",
            event.source.len(),
            event.source,
            id.namespace.len(),
            id.namespace,
            id.kind,
            id.external_id
        )
    })
}
fn chat(event: &LiveEvent) -> bool {
    matches!(event.kind, EventKind::Chat { .. })
}
impl EventScheduler {
    pub(super) fn prioritize(&self, candidates: &mut Vec<LiveEvent>, now: u64) -> Vec<String> {
        let alternatives = candidates
            .iter()
            .any(|e| key(e).is_none_or(|k| self.streaks.get(&k).copied().unwrap_or(0) < 2));
        if alternatives {
            candidates
                .retain(|e| key(e).is_none_or(|k| self.streaks.get(&k).copied().unwrap_or(0) < 2));
        }
        // Joint replies can saturate every present viewer. Break that tie with
        // one oldest viewer instead of exempting the entire room. Completing
        // that turn resets the others' streaks, so the next turn rotates away.
        else if candidates
            .iter()
            .filter_map(key)
            .collect::<HashSet<_>>()
            .len()
            > 1
        {
            let ordinary_due = self.non_chat_rounds >= 2 && candidates.iter().any(chat);
            let target = candidates
                .iter()
                .min_by_key(|e| (!(ordinary_due && chat(e)), e.occurred_at_ms, &e.id))
                .and_then(key);
            candidates.retain(|e| key(e) == target);
        }
        let forced = self.non_chat_rounds >= 2 && candidates.iter().any(chat);
        let focused = |e: &LiveEvent| {
            self.focus.as_ref().is_some_and(|(who, until, question)| {
                now < *until
                    && key(e).as_ref() == Some(who)
                    && match &e.kind {
                        EventKind::Chat { text } => {
                            let chars: Vec<_> =
                                question.chars().filter(|c| c.is_alphanumeric()).collect();
                            chars
                                .windows(2)
                                .any(|pair| text.contains(&pair.iter().collect::<String>()))
                        }
                        _ => false,
                    }
            })
        };
        let narrow_exists = candidates.iter().any(|e| self.narrowed.contains(&e.id));
        if narrow_exists {
            candidates.retain(|e| self.narrowed.contains(&e.id));
        }
        candidates.sort_by_key(|e| {
            (
                !(forced && chat(e)),
                !focused(e),
                !matches!(e.kind, EventKind::Gift { .. }),
                e.occurred_at_ms,
            )
        });
        candidates
            .iter()
            .filter(|e| (forced && chat(e)) || focused(e) || self.narrowed.contains(&e.id))
            .take(1)
            .map(|e| e.id.clone())
            .collect()
    }

    pub fn skipped(&mut self, ids: &[String], targets: &[String]) {
        self.narrowed.clear();
        for id in ids {
            let retries = self.retries.entry(id.clone()).or_default();
            let status = if *retries < 2 {
                *retries += 1;
                EventStatus::Pending
            } else {
                EventStatus::Skipped
            };
            if status == EventStatus::Pending && targets.contains(id) {
                self.narrowed.push(id.clone());
            }
            self.update(std::slice::from_ref(id), status, None, None);
        }
        self.retries.retain(|id, _| {
            self.records
                .iter()
                .any(|r| r.event.id == *id && !r.status.is_terminal())
        });
    }

    pub fn completed(&mut self, events: &[LiveEvent], assistant: &str, now: u64) {
        let keys: HashSet<_> = events.iter().filter_map(key).collect();
        self.streaks.retain(|k, _| keys.contains(k));
        for k in &keys {
            let count = self.streaks.entry(k.clone()).or_default();
            *count = count.saturating_add(1);
        }
        self.non_chat_rounds = if events.iter().any(chat) {
            0
        } else {
            self.non_chat_rounds.saturating_add(1)
        };
        if self
            .focus
            .as_ref()
            .is_some_and(|(who, until, _)| now >= *until || keys.contains(who))
        {
            self.focus = None;
        }
        if keys.len() == 1
            && events.iter().all(|e| key(e).is_some())
            && assistant.trim_end().ends_with(['?', '？'])
        {
            self.focus = Some((
                keys.into_iter().next().unwrap(),
                now.saturating_add(120_000),
                assistant.to_owned(),
            ));
        }
    }

    pub fn clear_context(&mut self) {
        self.focus = None;
        self.narrowed.clear();
        self.retries.clear();
    }
}
