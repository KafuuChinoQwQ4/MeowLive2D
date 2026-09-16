//! 纯应用状态机：调度事件、隔离在途决策、关联播放结果与已完成对话。
mod decisions;
mod playback;
mod settings;
mod types;
pub use settings::{AgentLimits, AgentSettings};
pub use types::{
    AgentPhase, AgentView, DecisionWork, EventRecord, EventStatus, PreparedSpeech, SubmitOutcome,
};

use crate::{
    ports::llm::{ConversationTurn, DecisionRequest},
    scheduler::{EventBatch, EventScheduler},
};
use meowlive_domain::event::LiveEvent;
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
    last_error: Option<String>,
    next_decision_ms: u64,
    proactive_after_ms: u64,
}
#[derive(Clone)]
struct Flight {
    id: u64,
    batch: EventBatch,
}
#[derive(Clone)]
struct ActiveSpeech {
    id: String,
    event_ids: Vec<String>,
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
            last_error: None,
            next_decision_ms: 0,
            proactive_after_ms: 0,
        })
    }

    pub fn submit(&mut self, event: LiveEvent, now_ms: u64) -> Result<SubmitOutcome, String> {
        self.scheduler.submit(event, now_ms)
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
            || self.flight.is_some()
            || self.current.is_some()
            || now_ms < self.next_decision_ms
        {
            return None;
        }
        let batch = self.scheduler.select(now_ms);
        if batch.events.is_empty()
            && (!self.settings.proactive_enabled || now_ms < self.proactive_after_ms)
        {
            return None;
        }
        self.next_work_id = self.next_work_id.checked_add(1)?;
        let work = DecisionWork {
            id: self.next_work_id,
            request: DecisionRequest {
                persona: self.settings.persona.clone(),
                topic: self.settings.topic.clone(),
                events: batch.events.clone(),
                history: self.history.iter().cloned().collect(),
            },
        };
        self.flight = Some(Flight { id: work.id, batch });
        self.last_error = None;
        Some(work)
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
