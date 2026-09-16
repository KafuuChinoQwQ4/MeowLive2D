use meowlive_application::agent::{AgentPhase, AgentView, EventStatus};
use meowlive_domain::event::{EventKind, LiveEvent};
use meowlive_protocol::agent as dto;

pub(super) fn event(input: dto::LiveEventInput, now_ms: u64) -> LiveEvent {
    LiveEvent {
        id: input.id,
        source: input.source,
        viewer: input.viewer,
        occurred_at_ms: now_ms,
        kind: match input.kind {
            dto::EventPayload::Chat { text } => EventKind::Chat { text },
            dto::EventPayload::Gift { name, count } => EventKind::Gift { name, count },
        },
    }
}

pub(super) fn snapshot(view: AgentView, configured: bool, connected: bool) -> dto::AgentSnapshot {
    dto::AgentSnapshot {
        paused: view.paused,
        phase: match view.phase {
            AgentPhase::Paused => dto::AgentPhase::Paused,
            AgentPhase::Waiting => dto::AgentPhase::Waiting,
            AgentPhase::Deciding => dto::AgentPhase::Deciding,
            AgentPhase::Speaking => dto::AgentPhase::Speaking,
        },
        settings: dto::AgentSettings {
            persona: view.settings.persona,
            topic: view.settings.topic,
            proactive_enabled: view.settings.proactive_enabled,
            cooldown_ms: view.settings.cooldown_ms as u32,
        },
        events: view
            .events
            .into_iter()
            .map(|r| dto::AgentEventSnapshot {
                event: dto::LiveEventInput {
                    id: r.event.id,
                    source: r.event.source,
                    viewer: r.event.viewer,
                    kind: match r.event.kind {
                        EventKind::Chat { text } => dto::EventPayload::Chat { text },
                        EventKind::Gift { name, count } => dto::EventPayload::Gift { name, count },
                    },
                },
                status: match r.status {
                    EventStatus::Pending => dto::AgentEventStatus::Pending,
                    EventStatus::Deciding => dto::AgentEventStatus::Deciding,
                    EventStatus::Skipped => dto::AgentEventStatus::Skipped,
                    EventStatus::Expired => dto::AgentEventStatus::Expired,
                    EventStatus::Queued => dto::AgentEventStatus::Queued,
                    EventStatus::Synthesizing => dto::AgentEventStatus::Synthesizing,
                    EventStatus::Ready => dto::AgentEventStatus::Ready,
                    EventStatus::Playing => dto::AgentEventStatus::Playing,
                    EventStatus::Completed => dto::AgentEventStatus::Completed,
                    EventStatus::Cancelled => dto::AgentEventStatus::Cancelled,
                    EventStatus::Failed => dto::AgentEventStatus::Failed,
                    EventStatus::Unknown => dto::AgentEventStatus::Unknown,
                },
                speech_id: r.speech_id,
                error: r.error,
            })
            .collect(),
        last_error: view.last_error,
        current_speech_id: view.current_speech_id,
        llm_configured: configured,
        bridge_connected: connected,
    }
}
