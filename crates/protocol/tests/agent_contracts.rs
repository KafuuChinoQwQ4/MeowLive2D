use meowlive_protocol::agent::{
    AgentEventSnapshot, AgentEventStatus, AgentPhase, AgentSettings, AgentSnapshot,
    EventBatchRequest, EventBatchResult, EventPayload, GiftMetadataInput, LiveEventInput,
    ViewerIdentityInput, ViewerIdentityKind,
};
use serde_json::json;

fn chat_event() -> LiveEventInput {
    LiveEventInput {
        id: "event-1".into(),
        source: "simulator".into(),
        viewer: "小猫".into(),
        viewer_identity: None,
        gift_metadata: None,
        kind: EventPayload::Chat {
            text: "晚上好".into(),
        },
    }
}

#[test]
fn agent_input_accepts_optional_identity_and_raw_gift_metadata() {
    let event: LiveEventInput = serde_json::from_value(json!({
        "id": "gift-1",
        "source": "simulator",
        "viewer": "观众甲",
        "viewer_identity": {
            "namespace": "simulator",
            "kind": "uid",
            "external_id": "42"
        },
        "gift_metadata": {
            "price": 1000,
            "paid": true,
            "medal_level": 12,
            "guard_level": 3
        },
        "kind": { "type": "gift", "name": "小鱼干", "count": 2 }
    }))
    .unwrap();
    assert_eq!(
        event.viewer_identity,
        Some(ViewerIdentityInput {
            namespace: "simulator".into(),
            kind: ViewerIdentityKind::Uid,
            external_id: "42".into(),
        })
    );
    assert_eq!(
        event.gift_metadata,
        Some(GiftMetadataInput {
            price: Some(1_000),
            paid: Some(true),
            medal_level: Some(12),
            guard_level: Some(3),
        })
    );
}

#[test]
fn agent_inputs_use_the_public_json_shape() {
    let settings: AgentSettings = serde_json::from_value(json!({
        "persona": "温柔的猫娘主播",
        "topic": "轻松聊天",
        "proactive_enabled": true,
        "cooldown_ms": 30_000
    }))
    .unwrap();
    assert_eq!(settings.cooldown_ms, 30_000);

    let batch: EventBatchRequest = serde_json::from_value(json!({
        "events": [{
            "id": "gift-1",
            "source": "simulator",
            "viewer": "观众甲",
            "kind": { "type": "gift", "name": "小鱼干", "count": 2 }
        }]
    }))
    .unwrap();
    assert_eq!(
        batch.events[0].kind,
        EventPayload::Gift {
            name: "小鱼干".into(),
            count: 2
        }
    );
}

#[test]
fn agent_inputs_reject_unknown_fields() {
    let settings = serde_json::from_value::<AgentSettings>(json!({
        "persona": "猫娘",
        "topic": "聊天",
        "proactive_enabled": false,
        "cooldown_ms": 1_000,
        "api_key": "must-not-cross-this-boundary"
    }));
    assert!(settings.is_err());

    let event = serde_json::from_value::<LiveEventInput>(json!({
        "id": "event-1",
        "source": "simulator",
        "viewer": "小猫",
        "kind": { "type": "chat", "text": "你好" },
        "occurred_at": 123
    }));
    assert!(event.is_err());

    let payload = serde_json::from_value::<EventPayload>(json!({
        "type": "chat",
        "text": "你好",
        "extra": true
    }));
    assert!(payload.is_err());
}

#[test]
fn agent_snapshot_serializes_only_public_state() {
    let snapshot = AgentSnapshot {
        paused: false,
        phase: AgentPhase::Speaking,
        settings: AgentSettings {
            interaction: Default::default(),
            persona: "猫娘".into(),
            topic: "游戏".into(),
            proactive_enabled: false,
            cooldown_ms: 30_000,
        },
        events: vec![AgentEventSnapshot {
            event: chat_event(),
            status: AgentEventStatus::Playing,
            speech_id: Some("speech-1".into()),
            error: None,
        }],
        last_error: None,
        current_speech_id: Some("speech-1".into()),
        llm_configured: true,
        bridge_connected: true,
    };

    assert_eq!(
        serde_json::to_value(snapshot).unwrap(),
        json!({
            "paused": false,
            "phase": "speaking",
            "settings": {
                "persona": "猫娘",
                "topic": "游戏",
                "proactive_enabled": false,
                "cooldown_ms": 30_000,
                "interaction": {
                    "chat_read_mode": "auto",
                    "welcome_enabled": true,
                    "busy_chat_count": 6,
                    "busy_enter_count": 3,
                    "busy_pending_count": 4,
                    "welcome_cooldown_ms": 30_000,
                    "welcome_viewer_cooldown_ms": 600_000
                }
            },
            "events": [{
                "event": {
                    "id": "event-1",
                    "source": "simulator",
                    "viewer": "小猫",
                    "kind": { "type": "chat", "text": "晚上好" }
                },
                "status": "playing",
                "speech_id": "speech-1",
                "error": null
            }],
            "last_error": null,
            "current_speech_id": "speech-1",
            "llm_configured": true,
            "bridge_connected": true
        })
    );

    assert_eq!(
        serde_json::to_value(EventBatchResult {
            accepted: 2,
            duplicates: 1,
            persisted: None,
            unscheduled: None,
        })
        .unwrap(),
        json!({ "accepted": 2, "duplicates": 1 })
    );
}
