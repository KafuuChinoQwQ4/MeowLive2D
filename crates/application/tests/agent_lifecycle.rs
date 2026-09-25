mod agent_support;
use agent_support::{answer, chat, session, speech};
use meowlive_application::agent::{
    AgentPhase, AgentSettings, AgentWaitReason, BeginDecision, EventStatus,
};
use meowlive_domain::speech::SpeechStatus;

#[test]
fn only_completed_playback_enters_history_and_releases_cooldown() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    assert!(work.request.history.is_empty());
    assert_eq!(
        agent
            .resolve(work.id, answer(&["one"]), "speech".into(), 1)
            .unwrap()
            .unwrap()
            .text,
        "你好 one。你好呀"
    );
    agent.submit(chat("two", 1), 1).unwrap();
    for (status, expected) in [
        (SpeechStatus::Queued, EventStatus::Queued),
        (SpeechStatus::Synthesizing, EventStatus::Synthesizing),
        (SpeechStatus::Ready, EventStatus::Ready),
        (SpeechStatus::Playing, EventStatus::Playing),
    ] {
        agent.sync_speech(&speech("speech", status), 2000);
        assert_eq!(agent.view(2000).events[0].status, expected);
        assert!(agent.begin(2000).is_none());
        assert_eq!(agent.view(2000).phase, AgentPhase::Speaking);
    }
    agent.sync_speech(&speech("speech", SpeechStatus::Completed), 2000);
    assert!(agent.begin(2999).is_none());
    let next = agent.begin(3000).unwrap();
    assert_eq!(next.request.history.len(), 1);
    assert!(next.request.history[0].user.contains("小猫"));
    assert!(next.request.history[0].user.contains("你好 one"));
    assert_eq!(next.request.history[0].assistant, "你好 one。你好呀");
}

#[test]
fn failed_cancelled_unknown_and_late_completion_never_create_memory() {
    for status in [
        SpeechStatus::Failed,
        SpeechStatus::Cancelled,
        SpeechStatus::Unknown,
    ] {
        let mut agent = session();
        agent.submit(chat("one", 0), 0).unwrap();
        agent.set_paused(false, 0);
        let work = agent.begin(0).unwrap();
        agent
            .resolve(work.id, answer(&["one"]), "speech".into(), 0)
            .unwrap();
        agent.sync_speech(&speech("unrelated", SpeechStatus::Completed), 1);
        assert_eq!(agent.view(1).current_speech_id.as_deref(), Some("speech"));
        agent.sync_speech(&speech("speech", status), 1);
        agent.sync_speech(&speech("speech", SpeechStatus::Completed), 2);
        agent.submit(chat("two", 2), 2).unwrap();
        assert!(agent.begin(1001).unwrap().request.history.is_empty());
    }
}

#[test]
fn pause_preserves_submitted_speech_while_stop_cancels_pending() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    agent
        .resolve(work.id, answer(&["one"]), "speech".into(), 0)
        .unwrap();
    agent.submit(chat("two", 1), 1).unwrap();
    agent.set_paused(true, 1);
    assert_eq!(agent.view(1).events[0].status, EventStatus::Queued);
    assert_eq!(agent.view(1).events[1].status, EventStatus::Pending);
    agent.stop(1);
    assert_eq!(agent.view(1).events[0].status, EventStatus::Queued);
    assert_eq!(agent.view(1).events[1].status, EventStatus::Cancelled);
    agent.sync_speech(&speech("speech", SpeechStatus::Cancelled), 1);
    assert!(agent.view(1).current_speech_id.is_none());
}

#[test]
fn configure_cancels_old_decision_and_keeps_waiting_events() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    agent.submit(chat("two", 1), 1).unwrap();
    agent
        .configure(
            AgentSettings {
                persona: "新的主播".into(),
                cooldown_ms: 1000,
                ..AgentSettings::default()
            },
            1,
        )
        .unwrap();
    assert!(agent.view(1).paused);
    assert!(
        agent
            .resolve(work.id, answer(&["one"]), "late".into(), 1)
            .is_err()
    );
    agent.set_paused(false, 2);
    let next = agent.begin(2).unwrap();
    assert_eq!(next.request.persona, "新的主播");
    assert_eq!(next.request.events[0].id, "two");
}

#[test]
fn proactive_waits_after_resume_but_first_event_runs_immediately() {
    let mut agent = session();
    let mut settings = agent.view(0).settings;
    settings.proactive_enabled = true;
    agent.configure(settings, 0).unwrap();
    agent.set_paused(false, 100);
    assert!(agent.begin(1099).is_none());
    let work = agent.begin(1100).unwrap();
    assert!(work.request.events.is_empty());
    agent
        .resolve(work.id, answer(&[]), "speech".into(), 1100)
        .unwrap();
    assert_eq!(
        agent.view(1100).current_speech_id.as_deref(),
        Some("speech")
    );
    let mut agent = session();
    agent.set_paused(false, 100);
    agent.submit(chat("one", 101), 101).unwrap();
    assert!(agent.begin(101).is_some());
}

#[test]
fn enqueue_failure_releases_current_speech_and_marks_event_failed() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    agent
        .resolve(work.id, answer(&["one"]), "speech".into(), 0)
        .unwrap();
    agent.speech_failed("speech", "queue unavailable".into(), 10);
    let view = agent.view(10);
    assert!(view.current_speech_id.is_none());
    assert_eq!(view.events[0].status, EventStatus::Failed);
    assert_eq!(view.last_error.as_deref(), Some("queue unavailable"));
}

#[test]
fn cancellation_before_enqueue_releases_current_speech_without_creating_history() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    agent
        .resolve(work.id, answer(&["one"]), "speech".into(), 0)
        .unwrap();
    agent.speech_cancelled("speech", 10);
    let view = agent.view(10);
    assert!(view.current_speech_id.is_none());
    assert_eq!(view.events[0].status, EventStatus::Cancelled);
    agent.submit(chat("two", 10), 10).unwrap();
    assert!(agent.begin(1010).unwrap().request.history.is_empty());
}

#[test]
fn begin_decision_explains_why_agent_is_waiting() {
    let mut agent = session();
    assert_eq!(
        agent.begin_decision(0),
        BeginDecision::Waiting(AgentWaitReason::Paused)
    );
    agent.set_paused(false, 0);
    assert_eq!(
        agent.begin_decision(0),
        BeginDecision::Waiting(AgentWaitReason::NoEligibleEvents)
    );
    agent.submit(chat("one", 0), 0).unwrap();
    let work = match agent.begin_decision(0) {
        BeginDecision::Work(work) => work,
        other => panic!("expected work, got {other:?}"),
    };
    assert_eq!(
        agent.begin_decision(0),
        BeginDecision::Waiting(AgentWaitReason::DecisionInFlight)
    );
    agent
        .resolve(work.id, answer(&["one"]), "speech".into(), 0)
        .unwrap();
    assert_eq!(
        agent.begin_decision(1),
        BeginDecision::Waiting(AgentWaitReason::SpeechInFlight)
    );
    agent.sync_speech(&speech("speech", SpeechStatus::Completed), 100);
    assert_eq!(
        agent.begin_decision(999),
        BeginDecision::Waiting(AgentWaitReason::Cooldown { remaining_ms: 101 })
    );
}

#[test]
fn waiting_reason_and_claim_both_reject_ineligible_welcomes() {
    let mut agent = session();
    let mut settings = agent.view(0).settings;
    settings.interaction.welcome_enabled = false;
    agent.configure(settings, 0).unwrap();
    agent
        .submit(
            meowlive_domain::event::LiveEvent {
                kind: meowlive_domain::event::EventKind::RoomEnter,
                ..chat("welcome", 0)
            },
            0,
        )
        .unwrap();
    agent.set_paused(false, 0);
    assert_eq!(
        agent.waiting_reason(0),
        Some(AgentWaitReason::NoEligibleEvents)
    );
    assert_eq!(
        agent.begin_decision(0),
        BeginDecision::Waiting(AgentWaitReason::NoEligibleEvents)
    );
    assert_eq!(agent.view(0).events[0].status, EventStatus::Skipped);
}
