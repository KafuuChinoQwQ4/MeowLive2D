//! 纯应用状态机：调度事件、隔离在途决策、关联播放结果与已完成对话。
mod decisions;
mod interaction;
mod playback;
mod settings;
mod types;
pub(crate) use interaction::read_prefix as reading_prefix;
pub use interaction::{ChatReadMode, InteractionSettings};
pub use settings::{AgentLimits, AgentSettings};
pub use types::{
    AgentPhase, AgentView, CompletedInteraction, DecisionWork, EventRecord, EventStatus,
    PreparedSpeech, SubmitOutcome,
};

use crate::{
    ports::llm::{ConversationTurn, DecisionRequest},
    scheduler::{EventBatch, EventScheduler},
};
use meowlive_domain::event::{EventKind, LiveEvent};
use std::collections::VecDeque;

#[derive(Clone)]
pub struct AgentSession {
    settings: AgentSettings,
    scheduler: EventScheduler,
    paused: bool,
    next_work_id: u64,
    flight: Option<Flight>,
    current: Option<ActiveSpeech>,
    history: VecDeque<ConversationTurn>,
    completed: Vec<CompletedInteraction>,
    last_error: Option<String>,
    next_decision_ms: u64,
    proactive_after_ms: u64,
    interaction: interaction::InteractionState,
}
#[derive(Clone)]
struct Flight {
    id: u64,
    batch: EventBatch,
    required_read: bool,
}
#[derive(Clone)]
struct ActiveSpeech {
    id: String,
    event_ids: Vec<String>,
    events: Vec<LiveEvent>,
    turn: ConversationTurn,
}

impl AgentSession {
    pub fn new(settings: AgentSettings, limits: AgentLimits) -> Result<Self, String> {
        settings.validate()?;
        limits.validate()?;
        Ok(Self {
            settings,
            scheduler: EventScheduler::new(limits),
            paused: true,
            next_work_id: 0,
            flight: None,
            current: None,
            history: VecDeque::new(),
            completed: Vec::new(),
            last_error: None,
            next_decision_ms: 0,
            proactive_after_ms: 0,
            interaction: interaction::InteractionState::default(),
        })
    }

    pub fn submit(&mut self, event: LiveEvent, now_ms: u64) -> Result<SubmitOutcome, String> {
        let outcome = self.scheduler.submit(event.clone(), now_ms)?;
        if outcome == SubmitOutcome::Accepted
            && now_ms
                < event
                    .occurred_at_ms
                    .saturating_add(event.response_ttl_ms(self.scheduler.limits.event_ttl_ms))
        {
            self.interaction.observe(&event, now_ms);
        }
        Ok(outcome)
    }

    /// Preserve the source event's elapsed UTC age when crossing into the
    /// process-relative scheduler, including ages greater than process uptime.
    pub fn submit_with_age(
        &mut self,
        event: LiveEvent,
        now_ms: u64,
        age_ms: u64,
    ) -> Result<SubmitOutcome, String> {
        let lifetime = event.response_ttl_ms(self.scheduler.limits.event_ttl_ms);
        let outcome = self
            .scheduler
            .submit_with_age(event.clone(), now_ms, age_ms)?;
        if outcome == SubmitOutcome::Accepted && age_ms < lifetime {
            self.interaction.observe(&event, now_ms);
        }
        Ok(outcome)
    }

    pub fn configure(&mut self, settings: AgentSettings, now_ms: u64) -> Result<(), String> {
        settings.validate()?;
        self.set_paused(true, now_ms);
        self.settings = settings;
        Ok(())
    }

    pub fn set_paused(&mut self, paused: bool, now_ms: u64) {
        self.scheduler.expire(now_ms);
        if paused {
            self.cancel_flight();
        } else if self.paused {
            self.proactive_after_ms = now_ms.saturating_add(self.settings.cooldown_ms);
        }
        self.paused = paused;
    }

    pub fn stop(&mut self, now_ms: u64) {
        self.set_paused(true, now_ms);
        self.scheduler.cancel_pending();
    }

    pub fn begin(&mut self, now_ms: u64) -> Option<DecisionWork> {
        self.scheduler.expire(now_ms);
        if self.paused
            || self.completed.len() >= self.scheduler.limits.history_limit
            || self.flight.is_some()
            || self.current.is_some()
            || now_ms < self.next_decision_ms
        {
            return None;
        }
        let pending = self
            .scheduler
            .records
            .iter()
            .filter(|r| r.status == EventStatus::Pending)
            .count();
        let busy = self
            .interaction
            .busy(&self.settings.interaction, pending, now_ms);
        let more_important = self.scheduler.records.iter().any(|r| {
            r.status == EventStatus::Pending && !matches!(r.event.kind, EventKind::RoomEnter)
        });
        let skipped_welcomes: Vec<_> = self
            .scheduler
            .records
            .iter()
            .filter(|r| {
                r.status == EventStatus::Pending && matches!(r.event.kind, EventKind::RoomEnter)
            })
            .filter(|r| {
                busy || more_important
                    || !self
                        .interaction
                        .may_welcome(&r.event, &self.settings.interaction, now_ms)
            })
            .map(|r| r.event.id.clone())
            .collect();
        self.scheduler.update(
            &skipped_welcomes,
            EventStatus::Skipped,
            None,
            Some("欢迎已跳过：关闭、繁忙或冷却中"),
        );
        let read_all = self.settings.interaction.chat_read_mode == ChatReadMode::All
            || (self.settings.interaction.chat_read_mode == ChatReadMode::Auto && !busy);
        let batch = self.scheduler.select(now_ms, read_all);
        if batch.events.is_empty()
            && (!self.settings.proactive_enabled || now_ms < self.proactive_after_ms)
        {
            return None;
        }
        self.next_work_id = self.next_work_id.checked_add(1)?;
        let work = DecisionWork {
            id: self.next_work_id,
            request: DecisionRequest {
                memory_context: vec![],
                persona: self.settings.persona.clone(),
                topic: self.settings.topic.clone(),
                events: batch.events.clone(),
                history: self.history.iter().cloned().collect(),
            },
        };
        let required_read = batch.events.len() == 1
            && match batch.events[0].kind {
                EventKind::Chat { .. } => read_all,
                EventKind::SuperChat { .. } | EventKind::RoomEnter => true,
                _ => false,
            };
        self.flight = Some(Flight {
            id: work.id,
            batch,
            required_read,
        });
        self.last_error = None;
        Some(work)
    }

    /// Events selected by reply_to, bounded by batch_size, available before queue submission.
    pub fn prepared_events(&self, speech_id: &str) -> Vec<LiveEvent> {
        self.current
            .as_ref()
            .filter(|s| s.id == speech_id)
            .map(|s| s.events.clone())
            .unwrap_or_default()
    }

    /// Drain at most history_limit completions. A full buffer blocks begin rather than dropping facts.
    pub fn take_completed(&mut self) -> Vec<CompletedInteraction> {
        std::mem::take(&mut self.completed)
    }

    /// Fence old model results and return the speech the service must stop.
    /// Accepted event records and completion facts remain available.
    pub fn invalidate_context(&mut self, now_ms: u64) -> Option<String> {
        if let Some(flight) = self.flight.take() {
            let ids = flight
                .batch
                .events
                .iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>();
            self.scheduler
                .update(&ids, EventStatus::Pending, None, None);
        }
        self.scheduler.clear_context();
        self.history.clear();
        let stopped = self.current.take().map(|speech| {
            self.scheduler
                .update(&speech.event_ids, EventStatus::Pending, None, None);
            speech.id
        });
        self.cooldown(now_ms);
        stopped
    }

    pub fn view(&mut self, now_ms: u64) -> AgentView {
        self.scheduler.expire(now_ms);
        let phase = if self.paused {
            AgentPhase::Paused
        } else if self.current.is_some() {
            AgentPhase::Speaking
        } else if self.flight.is_some() {
            AgentPhase::Deciding
        } else {
            AgentPhase::Waiting
        };
        AgentView {
            paused: self.paused,
            phase,
            settings: self.settings.clone(),
            events: self.scheduler.records.iter().cloned().collect(),
            last_error: self.last_error.clone(),
            current_speech_id: self.current.as_ref().map(|speech| speech.id.clone()),
        }
    }

    fn cancel_flight(&mut self) {
        if let Some(flight) = self.flight.take() {
            let ids = flight
                .batch
                .events
                .iter()
                .map(|event| event.id.clone())
                .collect::<Vec<_>>();
            self.scheduler
                .update(&ids, EventStatus::Cancelled, None, None);
        }
    }

    fn cooldown(&mut self, now_ms: u64) {
        self.next_decision_ms = now_ms.saturating_add(self.settings.cooldown_ms);
    }
}
