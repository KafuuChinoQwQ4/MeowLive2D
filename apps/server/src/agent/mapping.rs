use meowlive_application::agent::{AgentPhase, AgentView, EventStatus};
use meowlive_domain::event::{
    EventKind, GiftMetadata, LiveEvent, ViewerIdentity, ViewerIdentityKind,
};
use meowlive_protocol::agent as dto;

pub(crate) fn interaction(
    value: dto::InteractionSettings,
) -> meowlive_application::agent::InteractionSettings {
    use meowlive_application::agent::{ChatReadMode, InteractionSettings};
    InteractionSettings {
        chat_read_mode: match value.chat_read_mode {
            dto::ChatReadMode::Auto => ChatReadMode::Auto,
            dto::ChatReadMode::All => ChatReadMode::All,
            dto::ChatReadMode::Selective => ChatReadMode::Selective,
        },
        welcome_enabled: value.welcome_enabled,
        busy_chat_count: value.busy_chat_count,
        busy_enter_count: value.busy_enter_count,
        busy_pending_count: value.busy_pending_count,
        welcome_cooldown_ms: value.welcome_cooldown_ms,
        welcome_viewer_cooldown_ms: value.welcome_viewer_cooldown_ms,
    }
}

pub(crate) fn interaction_dto(
    value: meowlive_application::agent::InteractionSettings,
) -> dto::InteractionSettings {
    use meowlive_application::agent::ChatReadMode;
    dto::InteractionSettings {
        chat_read_mode: match value.chat_read_mode {
            ChatReadMode::Auto => dto::ChatReadMode::Auto,
            ChatReadMode::All => dto::ChatReadMode::All,
            ChatReadMode::Selective => dto::ChatReadMode::Selective,
        },
        welcome_enabled: value.welcome_enabled,
        busy_chat_count: value.busy_chat_count,
        busy_enter_count: value.busy_enter_count,
        busy_pending_count: value.busy_pending_count,
        welcome_cooldown_ms: value.welcome_cooldown_ms,
        welcome_viewer_cooldown_ms: value.welcome_viewer_cooldown_ms,
    }
}

pub(super) fn event(input: dto::LiveEventInput, now_ms: u64) -> LiveEvent {
    LiveEvent {
        id: input.id,
        source: input.source,
        viewer: input.viewer,
        viewer_identity: input.viewer_identity.map(|identity| ViewerIdentity {
            namespace: identity.namespace,
            kind: match identity.kind {
                dto::ViewerIdentityKind::OpenId => ViewerIdentityKind::OpenId,
                dto::ViewerIdentityKind::Uid => ViewerIdentityKind::Uid,
            },
            external_id: identity.external_id,
        }),
        occurred_at_ms: now_ms,
        gift_metadata: input.gift_metadata.map(|metadata| GiftMetadata {
            price: metadata.price,
            paid: metadata.paid,
            medal_level: metadata.medal_level,
            guard_level: metadata.guard_level,
        }),
        kind: match input.kind {
            dto::EventPayload::Chat { text } => EventKind::Chat { text },
            dto::EventPayload::Gift { name, count } => EventKind::Gift { name, count },
            dto::EventPayload::SuperChat {
                text,
                amount_cny,
                start_at_ms,
                end_at_ms,
            } => EventKind::SuperChat {
                text,
                amount_cny,
                start_at_ms,
                end_at_ms,
            },
            dto::EventPayload::RoomEnter => EventKind::RoomEnter,
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
            interaction: interaction_dto(view.settings.interaction),
        },
        events: view
            .events
            .into_iter()
            .map(|r| dto::AgentEventSnapshot {
                event: dto::LiveEventInput {
                    id: r.event.id,
                    source: r.event.source,
                    viewer: r.event.viewer,
                    viewer_identity: None,
                    gift_metadata: None,
                    kind: match r.event.kind {
                        EventKind::Chat { text } => dto::EventPayload::Chat { text },
                        EventKind::Gift { name, count } => dto::EventPayload::Gift { name, count },
                        EventKind::SuperChat {
                            text,
                            amount_cny,
                            start_at_ms,
                            end_at_ms,
                        } => dto::EventPayload::SuperChat {
                            text,
                            amount_cny,
                            start_at_ms,
                            end_at_ms,
                        },
                        EventKind::RoomEnter => dto::EventPayload::RoomEnter,
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
