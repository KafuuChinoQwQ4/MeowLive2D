mod agent_support;
use agent_support::{answer, chat, gift, ignore, speech};
use meowlive_application::agent::{
    AgentLimits, AgentSession, AgentSettings, ChatReadMode, EventStatus,
};
use meowlive_domain::event::{EventKind, LiveEvent, ViewerIdentity, ViewerIdentityKind};
use meowlive_domain::speech::SpeechStatus;

fn session() -> AgentSession {
    configured(AgentSettings {
        cooldown_ms: 1000,
        ..Default::default()
    })
}
fn configured(settings: AgentSettings) -> AgentSession {
    AgentSession::new(settings, AgentLimits::default()).unwrap()
}

#[test]
fn custom_system_prompt_is_attached_to_model_work() {
    let mut settings = AgentSettings {
        cooldown_ms: 1000,
        system_prompt: "直接读出弹幕原文".into(),
        ..Default::default()
    };
    settings.interaction.chat_read_mode = ChatReadMode::All;
    let mut agent = configured(settings);
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);

    let work = agent.begin(0).unwrap();

    assert_eq!(work.request.system_prompt, "直接读出弹幕原文");
}

fn enter(id: &str, viewer: &str, at: u64) -> LiveEvent {
    LiveEvent {
        kind: EventKind::RoomEnter,
        viewer: viewer.into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "room".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: viewer.chars().map(|c| format!("{:x}-", c as u32)).collect(),
        }),
        ..chat(id, at)
    }
}
fn sc(id: &str, at: u64) -> LiveEvent {
    LiveEvent {
        kind: EventKind::SuperChat {
            text: "你最喜欢哪部电影？".into(),
            amount_cny: 30,
            start_at_ms: 1_800_000_000_000,
            end_at_ms: 1_800_000_300_000,
        },
        ..chat(id, at)
    }
}

#[test]
fn sparse_chat_reads_original_message_before_the_reply() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    let speech = agent
        .resolve(work.id, answer(&["one"]), "speech".into(), 0)
        .unwrap()
        .unwrap();
    assert!(!speech.text.contains("小猫说"));
    assert!(speech.text.starts_with("你好 one。"));
    assert!(speech.text.contains("你好 one"));
    assert!(speech.text.find("你好 one").unwrap() < speech.text.find("你好呀").unwrap());
}

#[test]
fn sparse_messages_are_read_in_individual_turns() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.submit(chat("two", 1), 1).unwrap();
    agent.set_paused(false, 1);
    let work = agent.begin(1).unwrap();
    assert_eq!(work.request.events.len(), 1);
    assert_eq!(work.request.events[0].id, "one");
}

#[test]
fn sparse_chat_cannot_be_silently_discarded_by_the_model() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    let prepared = agent
        .resolve(work.id, ignore(), "speech".into(), 0)
        .unwrap()
        .unwrap();
    assert!(prepared.text.contains("你好 one"));
    assert_eq!(agent.view(0).events[0].status, EventStatus::Queued);
}

#[test]
fn busy_chat_allows_selection_and_manual_all_overrides_it() {
    for mode in [
        ChatReadMode::Auto,
        ChatReadMode::All,
        ChatReadMode::Selective,
    ] {
        let mut settings = AgentSettings::default();
        settings.interaction.chat_read_mode = mode;
        let mut agent = configured(settings);
        for n in 0..6 {
            agent.submit(chat(&format!("m{n}"), 0), 0).unwrap();
        }
        agent.set_paused(false, 0);
        let work = agent.begin(0).unwrap();
        assert_eq!(
            work.request.events.len(),
            if mode == ChatReadMode::All { 1 } else { 6 }
        );
        let prepared = agent
            .resolve(work.id, ignore(), "speech".into(), 0)
            .unwrap();
        assert_eq!(prepared.is_some(), mode == ChatReadMode::All);
    }
}

#[test]
fn duplicate_events_do_not_make_sparse_room_busy() {
    let mut agent = session();
    for _ in 0..20 {
        agent.submit(chat("one", 0), 0).unwrap();
    }
    agent.submit(chat("two", 0), 0).unwrap();
    agent.set_paused(false, 0);
    assert_eq!(agent.begin(0).unwrap().request.events.len(), 1);
}

#[test]
fn selective_chat_mode_still_welcomes_one_viewer_and_obeys_cooldown() {
    let mut settings = AgentSettings {
        cooldown_ms: 1000,
        ..Default::default()
    };
    settings.interaction.chat_read_mode = ChatReadMode::Selective;
    let mut agent = configured(settings);
    agent.submit(enter("one", "甲", 0), 0).unwrap();
    agent.submit(enter("two", "乙", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    assert_eq!(work.request.events.len(), 1);
    let prepared = agent
        .resolve(work.id, ignore(), "welcome".into(), 0)
        .unwrap()
        .unwrap();
    assert!(prepared.text.contains("欢迎甲"));
    assert!(!prepared.text.contains("乙"));
    agent.sync_speech(&speech("welcome", SpeechStatus::Completed), 0);
    assert!(agent.begin(1000).is_none());
    assert_eq!(
        agent
            .view(1000)
            .events
            .iter()
            .find(|e| e.event.id == "two")
            .unwrap()
            .status,
        EventStatus::Skipped
    );
}

#[test]
fn superchat_has_its_own_priority_turn_and_retains_amount_and_full_text() {
    let mut agent = session();
    agent.submit(chat("chat", 0), 0).unwrap();
    agent.submit(gift("gift", 0, 1), 0).unwrap();
    agent.submit(sc("sc", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    assert_eq!(work.request.events.len(), 1);
    assert_eq!(work.request.events[0].id, "sc");
    let prepared = agent
        .resolve(work.id, answer(&["sc"]), "speech".into(), 0)
        .unwrap()
        .unwrap();
    assert!(prepared.text.starts_with("感谢小猫的30元SC。"));
    assert!(!prepared.text.contains("留言说"));
    assert!(!prepared.text.contains("？。"));
    assert!(prepared.text.contains("你最喜欢哪部电影？"));
    assert!(prepared.text.ends_with("你好呀"));
}

#[test]
fn superchat_survives_ordinary_ttl_but_expires_at_display_end() {
    let mut agent = session();
    agent.submit_with_age(sc("sc", 0), 0, 90_000).unwrap();
    agent.set_paused(false, 60_000);
    assert!(agent.begin(60_000).is_some());
    let mut expired = session();
    expired.submit_with_age(sc("sc", 0), 0, 300_000).unwrap();
    expired.set_paused(false, 0);
    assert!(expired.begin(0).is_none());
    assert_eq!(expired.view(0).events[0].status, EventStatus::Expired);
}

#[test]
fn sparse_entry_welcomes_by_name_and_repeat_identity_is_suppressed() {
    let mut agent = session();
    agent.submit(enter("first", "小鱼", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    let prepared = agent
        .resolve(work.id, answer(&["first"]), "speech".into(), 0)
        .unwrap()
        .unwrap();
    assert!(prepared.text.contains("欢迎小鱼"));
    assert!(prepared.text.contains("感谢"));
    agent.sync_speech(&speech("speech", SpeechStatus::Completed), 1000);
    let mut again = enter("again", "小鱼", 61_000);
    again.viewer = "改过昵称".into();
    agent.submit(again, 61_000).unwrap();
    assert!(agent.begin(61_000).is_none());
    assert_eq!(
        agent.view(61_000).events.last().unwrap().status,
        EventStatus::Skipped
    );
}

#[test]
fn burst_entries_are_skipped_without_stale_welcome_backlog() {
    let mut agent = session();
    for n in 0..3 {
        agent
            .submit(enter(&format!("e{n}"), &format!("观众{n}"), 0), 0)
            .unwrap();
    }
    agent.set_paused(false, 0);
    assert!(agent.begin(0).is_none());
    assert!(
        agent
            .view(0)
            .events
            .iter()
            .all(|r| r.status == EventStatus::Skipped)
    );
    assert!(agent.begin(61_000).is_none());
    agent
        .submit(enter("new", "新观众", 61_000), 61_000)
        .unwrap();
    assert!(agent.begin(61_000).is_some());
}

#[test]
fn pending_content_and_manual_welcome_disable_suppress_entries() {
    for disabled in [false, true] {
        let mut settings = AgentSettings::default();
        settings.interaction.welcome_enabled = !disabled;
        let mut agent = configured(settings);
        agent.submit(enter("entry", "观众", 0), 0).unwrap();
        if !disabled {
            agent.submit(chat("chat", 0), 0).unwrap();
        }
        agent.set_paused(false, 0);
        let work = agent.begin(0);
        assert_eq!(work.is_some(), !disabled);
        assert_eq!(agent.view(0).events[0].status, EventStatus::Skipped);
    }
}

#[test]
fn welcome_is_dropped_if_room_becomes_busy_during_generation() {
    let mut agent = session();
    agent.submit(enter("first", "甲", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    agent.submit(enter("second", "乙", 1), 1).unwrap();
    agent.submit(enter("third", "丙", 1), 1).unwrap();
    assert!(
        agent
            .resolve(work.id, answer(&["first"]), "speech".into(), 2)
            .unwrap()
            .is_none()
    );
    assert_eq!(agent.view(2).events[0].status, EventStatus::Skipped);
}

#[test]
fn full_ordinary_queue_reserves_admission_for_paid_text() {
    let mut agent = AgentSession::new(
        AgentSettings::default(),
        AgentLimits {
            pending_capacity: 1,
            ..Default::default()
        },
    )
    .unwrap();
    agent.submit(chat("chat", 0), 0).unwrap();
    assert!(agent.submit(sc("sc", 0), 0).is_ok());
    agent.set_paused(false, 0);
    assert_eq!(agent.begin(0).unwrap().request.events[0].id, "sc");
    assert_eq!(agent.view(0).events[0].status, EventStatus::Skipped);
}

#[test]
fn longest_original_and_reply_fit_broadcast_without_truncation() {
    let mut agent = session();
    let mut event = sc("sc", 0);
    if let EventKind::SuperChat { text, .. } = &mut event.kind {
        *text = "原".repeat(500);
    }
    agent.submit(event, 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    let mut decision = answer(&["sc"]);
    decision.text = Some("答".repeat(500));
    let prepared = agent
        .resolve(work.id, decision, "speech".into(), 0)
        .unwrap()
        .unwrap();
    assert_eq!(prepared.text.chars().filter(|c| *c == '原').count(), 500);
    assert_eq!(prepared.text.chars().filter(|c| *c == '答').count(), 500);
    let mut queue = meowlive_application::speech::SpeechQueue::new(2, 10);
    queue.set_connected(true);
    assert!(
        queue
            .enqueue_broadcast("speech", prepared.text, "default")
            .is_ok()
    );
}

#[test]
fn superchat_that_expires_during_generation_is_not_spoken() {
    let mut agent = session();
    agent.submit_with_age(sc("sc", 0), 0, 299_999).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    assert!(
        agent
            .resolve(work.id, answer(&["sc"]), "speech".into(), 2)
            .unwrap()
            .is_none()
    );
    assert_eq!(agent.view(2).events[0].status, EventStatus::Expired);
}
