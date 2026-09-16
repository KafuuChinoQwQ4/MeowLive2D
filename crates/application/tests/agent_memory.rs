mod agent_support;
use agent_support::{answer, chat, speech};
use meowlive_application::agent::{AgentLimits, AgentSession, AgentSettings};
use meowlive_domain::{event::EventKind, speech::SpeechStatus};

#[test]
fn remembers_only_bounded_recent_completed_turns_once() {
    let mut agent = AgentSession::new(
        AgentSettings {
            cooldown_ms: 1000,
            ..AgentSettings::default()
        },
        AgentLimits {
            conversation_limit: 2,
            ..AgentLimits::default()
        },
    )
    .unwrap();
    agent.set_paused(false, 0);
    for (index, id) in ["one", "two", "three"].iter().enumerate() {
        let now = index as u64 * 1000;
        agent.submit(chat(id, now), now).unwrap();
        let work = agent.begin(now).unwrap();
        agent
            .resolve(work.id, answer(&[id]), id.to_string(), now)
            .unwrap();
        agent.sync_speech(&speech(id, SpeechStatus::Completed), now);
        agent.sync_speech(&speech(id, SpeechStatus::Completed), now);
    }
    agent.submit(chat("four", 3000), 3000).unwrap();
    let work = agent.begin(3000).unwrap();
    assert_eq!(work.request.history.len(), 2);
    assert!(work.request.history[0].user.contains("two"));
    assert!(work.request.history[1].user.contains("three"));
}

#[test]
fn selected_context_is_bounded_without_including_skipped_events() {
    let mut agent = AgentSession::new(
        AgentSettings {
            cooldown_ms: 1000,
            ..AgentSettings::default()
        },
        AgentLimits {
            batch_size: 16,
            ..AgentLimits::default()
        },
    )
    .unwrap();
    let ids: Vec<_> = (0..16).map(|index| index.to_string()).collect();
    for id in &ids {
        let mut event = chat(id, 0);
        event.kind = EventKind::Chat {
            text: "猫".repeat(500),
        };
        agent.submit(event, 0).unwrap();
    }
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    let selected: Vec<_> = ids.iter().map(String::as_str).collect();
    agent
        .resolve(work.id, answer(&selected), "speech".into(), 0)
        .unwrap();
    agent.sync_speech(&speech("speech", SpeechStatus::Completed), 0);
    agent.submit(chat("next", 1000), 1000).unwrap();
    let next = agent.begin(1000).unwrap();
    assert_eq!(next.request.history[0].user.chars().count(), 4000);
}
