#![allow(dead_code)]
use meowlive_application::agent::{AgentLimits, AgentSession, AgentSettings};
use meowlive_application::ports::llm::AgentDecision;
use meowlive_domain::event::{EventKind, LiveEvent};
use meowlive_domain::speech::{SpeechStatus, SpeechTask, SpeechText};
use meowlive_domain::voice::VoiceId;

pub fn chat(id: &str, at: u64) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "simulator".into(),
        viewer: "小猫".into(),
        viewer_identity: None,
        occurred_at_ms: at,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: format!("你好 {id}"),
        },
    }
}
pub fn gift(id: &str, at: u64, count: u32) -> LiveEvent {
    LiveEvent {
        kind: EventKind::Gift {
            name: "鲜花".into(),
            count,
        },
        ..chat(id, at)
    }
}
pub fn session() -> AgentSession {
    AgentSession::new(
        AgentSettings {
            cooldown_ms: 1000,
            interaction: meowlive_application::agent::InteractionSettings {
                chat_read_mode: meowlive_application::agent::ChatReadMode::Selective,
                ..Default::default()
            },
            ..AgentSettings::default()
        },
        AgentLimits::default(),
    )
    .unwrap()
}
pub fn answer(ids: &[&str]) -> AgentDecision {
    AgentDecision {
        reply_to: ids.iter().map(|id| (*id).into()).collect(),
        text: Some("你好呀".into()),
        topic: None,
    }
}
pub fn ignore() -> AgentDecision {
    AgentDecision {
        reply_to: vec![],
        text: None,
        topic: None,
    }
}
pub fn speech(id: &str, status: SpeechStatus) -> SpeechTask {
    SpeechTask {
        id: id.into(),
        generation: 1,
        text: SpeechText::new("你好呀").unwrap(),
        voice_id: VoiceId::new("default").unwrap(),
        status,
        error: None,
    }
}

pub fn stable_gift(id: &str, at: u64, count: u32) -> LiveEvent {
    LiveEvent {
        viewer_identity: Some(meowlive_domain::event::ViewerIdentity {
            namespace: "room:1".into(),
            kind: meowlive_domain::event::ViewerIdentityKind::OpenId,
            external_id: "viewer:1".into(),
        }),
        ..gift(id, at, count)
    }
}
