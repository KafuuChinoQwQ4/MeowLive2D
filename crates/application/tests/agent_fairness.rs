mod agent_support;
use agent_support::*;
use meowlive_domain::{
    event::{LiveEvent, ViewerIdentity, ViewerIdentityKind},
    speech::SpeechStatus,
};
fn identified(mut e: LiveEvent, id: &str) -> LiveEvent {
    e.viewer_identity = Some(ViewerIdentity {
        namespace: "room:1".into(),
        kind: ViewerIdentityKind::OpenId,
        external_id: id.into(),
    });
    e
}
#[test]
fn anonymous_same_name_gifts_are_not_merged() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(gift("a", 0, 1), 0).unwrap();
    a.submit(gift("b", 0, 1), 0).unwrap();
    let w = a.begin(0).unwrap();
    assert!(a.resolve(w.id, answer(&["a"]), "s".into(), 0).is_ok());
}
#[test]
fn completed_two_turns_force_another_viewer_and_ordinary_chat() {
    let mut a = session();
    a.set_paused(false, 0);
    for i in 0..2 {
        let id = format!("g{i}");
        let now = i * 1000;
        a.submit(identified(gift(&id, now, 1), "donor"), now)
            .unwrap();
        let w = a.begin(now).unwrap();
        a.resolve(w.id, answer(&[&id]), id.clone(), now).unwrap();
        a.sync_speech(&speech(&id, SpeechStatus::Completed), now);
    }
    a.submit(identified(gift("g2", 2000, 1), "donor"), 2000)
        .unwrap();
    a.submit(identified(chat("ordinary", 2000), "other"), 2000)
        .unwrap();
    let w = a.begin(2000).unwrap();
    assert_eq!(w.request.events[0].id, "ordinary");
    assert!(!w.request.events.iter().any(|e| e.id == "g2"));
}
#[test]
fn skipped_question_gets_only_two_reselections() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(chat("q", 0), 0).unwrap();
    for i in 0..3 {
        let w = a.begin(i * 1000).expect("bounded retry");
        a.resolve(w.id, ignore(), "x".into(), i * 1000).unwrap();
    }
    assert!(a.begin(3000).is_none());
}
#[test]
fn completed_associations_are_selected_only_and_drained_once() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(chat("a", 0), 0).unwrap();
    a.submit(chat("b", 0), 0).unwrap();
    let w = a.begin(0).unwrap();
    a.resolve(w.id, answer(&["a"]), "s".into(), 0).unwrap();
    assert_eq!(
        a.prepared_events("s")
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a"]
    );
    assert!(a.take_completed().is_empty());
    a.sync_speech(&speech("s", SpeechStatus::Completed), 0);
    let done = a.take_completed();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].events[0].id, "a");
    assert_eq!(done[0].assistant, "小猫说：你好 a。你好呀");
    a.sync_speech(&speech("s", SpeechStatus::Completed), 0);
    assert!(a.take_completed().is_empty());
}
#[test]
fn invalidation_cancels_old_speech_and_fences_history() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(chat("a", 0), 0).unwrap();
    let w = a.begin(0).unwrap();
    a.resolve(w.id, answer(&["a"]), "s".into(), 0).unwrap();
    assert_eq!(a.invalidate_context(0), Some("s".into()));
    a.sync_speech(&speech("s", SpeechStatus::Completed), 0);
    assert!(a.take_completed().is_empty());
    a.submit(chat("b", 1000), 1000).unwrap();
    assert!(a.begin(1000).unwrap().request.history.is_empty());
}
#[test]
fn focused_related_reply_precedes_gifts_only_after_completion() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(identified(chat("a", 0), "one"), 0).unwrap();
    let w = a.begin(0).unwrap();
    let mut d = answer(&["a"]);
    d.text = Some("你喜欢橘猫吗？".into());
    a.resolve(w.id, d, "s".into(), 0).unwrap();
    a.sync_speech(&speech("s", SpeechStatus::Completed), 0);
    let mut follow = identified(chat("b", 1000), "one");
    follow.kind = meowlive_domain::event::EventKind::Chat {
        text: "我喜欢橘猫".into(),
    };
    a.submit(follow, 1000).unwrap();
    a.submit(identified(gift("g", 1000, 1), "two"), 1000)
        .unwrap();
    let w = a.begin(1000).unwrap();
    assert_eq!(w.request.events[0].id, "b");
    a.resolve(w.id, answer(&["g"]), "g".into(), 1000).unwrap();
    a.sync_speech(&speech("g", SpeechStatus::Completed), 1000);
    let w = a.begin(2000).unwrap();
    assert_eq!(w.request.events.len(), 1);
    assert_eq!(w.request.events[0].id, "b");
}
#[test]
fn failed_cancelled_and_unknown_playback_do_not_count_as_turns() {
    for status in [
        SpeechStatus::Failed,
        SpeechStatus::Cancelled,
        SpeechStatus::Unknown,
    ] {
        let mut a = session();
        a.set_paused(false, 0);
        for i in 0..2 {
            let id = format!("g{i}");
            let now = i * 1000;
            a.submit(identified(gift(&id, now, 1), "one"), now).unwrap();
            let w = a.begin(now).unwrap();
            a.resolve(w.id, answer(&[&id]), id.clone(), now).unwrap();
            a.sync_speech(&speech(&id, status), now);
        }
        a.submit(identified(gift("g2", 2000, 1), "one"), 2000)
            .unwrap();
        a.submit(identified(chat("c", 2000), "two"), 2000).unwrap();
        assert_eq!(a.begin(2000).unwrap().request.events[0].id, "g2");
        assert!(a.take_completed().is_empty());
    }
}
#[test]
fn same_name_different_identity_can_be_selected_independently() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(identified(gift("a", 0, 1), "one"), 0).unwrap();
    a.submit(identified(gift("b", 0, 1), "two"), 0).unwrap();
    let w = a.begin(0).unwrap();
    assert!(a.resolve(w.id, answer(&["a"]), "s".into(), 0).is_ok());
}
#[test]
fn ordinary_target_avoided_by_model_is_narrowed_next_turn() {
    let mut a = session();
    a.set_paused(false, 0);
    for i in 0..2 {
        let id = format!("g{i}");
        let now = i * 1000;
        a.submit(identified(gift(&id, now, 1), &id), now).unwrap();
        let w = a.begin(now).unwrap();
        a.resolve(w.id, answer(&[&id]), id.clone(), now).unwrap();
        a.sync_speech(&speech(&id, SpeechStatus::Completed), now);
    }
    a.submit(identified(gift("g2", 2000, 1), "three"), 2000)
        .unwrap();
    a.submit(identified(chat("c", 2000), "four"), 2000).unwrap();
    let w = a.begin(2000).unwrap();
    assert_eq!(w.request.events[0].id, "c");
    a.resolve(w.id, answer(&["g2"]), "s".into(), 2000).unwrap();
    a.sync_speech(&speech("s", SpeechStatus::Completed), 2000);
    a.submit(gift("flood", 3000, 1), 3000).unwrap();
    let w = a.begin(3000).unwrap();
    assert_eq!(w.request.events.len(), 1);
    assert_eq!(w.request.events[0].id, "c");
}
#[test]
fn blocks_public_internal_scores() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(chat("a", 0), 0).unwrap();
    let w = a.begin(0).unwrap();
    let mut d = answer(&["a"]);
    d.text = Some("你的好感度是80分".into());
    assert!(a.resolve(w.id, d, "s".into(), 0).is_err());
}
#[test]
fn undrained_completions_apply_backpressure_without_dropping_facts() {
    let mut a = meowlive_application::agent::AgentSession::new(
        meowlive_application::agent::AgentSettings {
            cooldown_ms: 1000,
            ..Default::default()
        },
        meowlive_application::agent::AgentLimits {
            history_limit: 1,
            ..Default::default()
        },
    )
    .unwrap();
    a.set_paused(false, 0);
    a.submit(chat("a", 0), 0).unwrap();
    let w = a.begin(0).unwrap();
    a.resolve(w.id, answer(&["a"]), "s".into(), 0).unwrap();
    a.sync_speech(&speech("s", SpeechStatus::Completed), 0);
    a.submit(chat("b", 1000), 1000).unwrap();
    assert!(a.begin(1000).is_none());
    assert_eq!(a.take_completed().len(), 1);
    assert!(a.begin(1000).is_some());
}
#[test]
fn focus_does_not_match_names_unrelated_messages_or_expired_windows() {
    for (identity, text, now) in [
        ("other", "我喜欢橘猫", 1000),
        ("one", "天气晴朗", 1000),
        ("one", "我喜欢橘猫", 120000),
    ] {
        let mut a = session();
        a.set_paused(false, 0);
        a.submit(identified(chat("a", 0), "one"), 0).unwrap();
        let w = a.begin(0).unwrap();
        let mut d = answer(&["a"]);
        d.text = Some("喜欢橘猫吗？".into());
        a.resolve(w.id, d, "s".into(), 0).unwrap();
        a.sync_speech(&speech("s", SpeechStatus::Completed), 0);
        let mut c = identified(chat("c", now), identity);
        c.kind = meowlive_domain::event::EventKind::Chat { text: text.into() };
        a.submit(c, now).unwrap();
        a.submit(identified(gift("g", now, 1), "donor"), now)
            .unwrap();
        assert_eq!(a.begin(now).unwrap().request.events[0].id, "g");
    }
}
#[test]
fn single_viewer_can_continue_after_two_completed_turns() {
    let mut a = session();
    a.set_paused(false, 0);
    for i in 0..4 {
        let id = format!("c{i}");
        let now = i * 1000;
        a.submit(identified(chat(&id, now), "one"), now).unwrap();
        let w = a.begin(now).unwrap();
        a.resolve(w.id, answer(&[&id]), id.clone(), now).unwrap();
        a.sync_speech(&speech(&id, SpeechStatus::Completed), now);
    }
    assert_eq!(a.take_completed().len(), 4);
}
#[test]
fn focus_survives_unrelated_completed_thanks_within_window() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(identified(chat("a", 0), "one"), 0).unwrap();
    let w = a.begin(0).unwrap();
    let mut d = answer(&["a"]);
    d.text = Some("喜欢橘猫吗？".into());
    a.resolve(w.id, d, "s".into(), 0).unwrap();
    a.sync_speech(&speech("s", SpeechStatus::Completed), 0);
    a.submit(identified(gift("g", 1000, 1), "other"), 1000)
        .unwrap();
    let w = a.begin(1000).unwrap();
    a.resolve(w.id, answer(&["g"]), "s2".into(), 1000).unwrap();
    a.sync_speech(&speech("s2", SpeechStatus::Completed), 1000);
    let mut c = identified(chat("c", 2000), "one");
    c.kind = meowlive_domain::event::EventKind::Chat {
        text: "我喜欢橘猫".into(),
    };
    a.submit(c, 2000).unwrap();
    a.submit(identified(gift("g2", 2000, 1), "third"), 2000)
        .unwrap();
    assert_eq!(a.begin(2000).unwrap().request.events[0].id, "c");
}
#[test]
fn jointly_saturated_viewers_rotate_through_a_single_target() {
    let mut a = session();
    a.set_paused(false, 0);
    for round in 0..2 {
        let now = round * 1000;
        let ids = [format!("a{round}"), format!("b{round}")];
        for (event, viewer) in ids.iter().zip(["one", "two"]) {
            a.submit(identified(chat(event, now), viewer), now).unwrap();
        }
        let w = a.begin(now).unwrap();
        a.resolve(w.id, answer(&[&ids[0], &ids[1]]), format!("s{round}"), now)
            .unwrap();
        a.sync_speech(&speech(&format!("s{round}"), SpeechStatus::Completed), now);
    }
    a.submit(identified(chat("a2", 2000), "one"), 2000).unwrap();
    a.submit(identified(chat("b2", 2000), "two"), 2000).unwrap();
    let w = a.begin(2000).unwrap();
    assert_eq!(
        w.request.events.len(),
        1,
        "joint saturation must not exempt all viewers"
    );
    let selected = w.request.events[0].id.clone();
    a.resolve(w.id, answer(&[&selected]), "rotate".into(), 2000)
        .unwrap();
    a.sync_speech(&speech("rotate", SpeechStatus::Completed), 2000);
    a.submit(identified(chat("a3", 3000), "one"), 3000).unwrap();
    a.submit(identified(chat("b3", 3000), "two"), 3000).unwrap();
    let next = a.begin(3000).unwrap();
    assert!(
        next.request
            .events
            .iter()
            .all(|e| e.viewer_identity != w.request.events[0].viewer_identity)
    );
}

#[test]
fn knowledge_change_requeues_live_work_without_losing_other_pending_events() {
    let mut a = session();
    a.set_paused(false, 0);
    a.submit(chat("old-context", 0), 0).unwrap();
    let old = a.begin(0).unwrap();
    a.submit(chat("waiting", 0), 0).unwrap();
    a.invalidate_context(0);
    assert!(
        a.resolve(old.id, answer(&["old-context"]), "stale".into(), 1)
            .is_err()
    );
    let fresh = a.begin(1000).unwrap();
    assert!(fresh.request.events.iter().any(|e| e.id == "old-context"));
    assert!(fresh.request.events.iter().any(|e| e.id == "waiting"));
    assert!(fresh.request.history.is_empty());
}
