use meowlive_application::agent::{AgentLimits, AgentSession, AgentSettings};

#[test]
fn validates_persona_topic_and_cooldown_before_mutating_configuration() {
    let default = AgentSettings::default();
    let mut agent = AgentSession::new(default.clone(), AgentLimits::default()).unwrap();
    for settings in [
        AgentSettings {
            persona: " ".into(),
            ..default.clone()
        },
        AgentSettings {
            persona: "猫".repeat(2001),
            ..default.clone()
        },
        AgentSettings {
            persona: "bad\0".into(),
            ..default.clone()
        },
        AgentSettings {
            system_prompt: "猫".repeat(4001),
            ..default.clone()
        },
        AgentSettings {
            topic: "猫".repeat(201),
            ..default.clone()
        },
        AgentSettings {
            topic: "bad\0".into(),
            ..default.clone()
        },
        AgentSettings {
            cooldown_ms: 999,
            ..default.clone()
        },
        AgentSettings {
            cooldown_ms: 3_600_001,
            ..default.clone()
        },
    ] {
        assert!(agent.configure(settings, 0).is_err());
        assert_eq!(agent.view(0).settings, default);
    }
    assert!(
        AgentSettings {
            persona: "猫".repeat(2000),
            topic: "猫".repeat(200),
            system_prompt: "猫".repeat(4000),
            cooldown_ms: 3_600_000,
            ..default
        }
        .validate()
        .is_ok()
    );
}

#[test]
fn rejects_unbounded_or_empty_operational_limits() {
    for limits in [
        AgentLimits {
            pending_capacity: 0,
            ..AgentLimits::default()
        },
        AgentLimits {
            pending_capacity: 513,
            ..AgentLimits::default()
        },
        AgentLimits {
            history_limit: 2001,
            ..AgentLimits::default()
        },
        AgentLimits {
            dedup_capacity: 4097,
            ..AgentLimits::default()
        },
        AgentLimits {
            batch_size: 17,
            ..AgentLimits::default()
        },
        AgentLimits {
            conversation_limit: 21,
            ..AgentLimits::default()
        },
        AgentLimits {
            event_ttl_ms: 999,
            ..AgentLimits::default()
        },
        AgentLimits {
            event_ttl_ms: 600_001,
            ..AgentLimits::default()
        },
        AgentLimits {
            gift_merge_ms: 10_001,
            ..AgentLimits::default()
        },
    ] {
        assert!(AgentSession::new(AgentSettings::default(), limits).is_err());
    }
    assert!(
        AgentSession::new(
            AgentSettings::default(),
            AgentLimits {
                history_limit: 1,
                gift_merge_ms: 0,
                ..AgentLimits::default()
            }
        )
        .is_ok()
    );
}
