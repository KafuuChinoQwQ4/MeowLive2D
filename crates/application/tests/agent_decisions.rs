mod agent_support;
use agent_support::{answer, chat, ignore, session};
use meowlive_application::agent::{AgentPhase, EventStatus};

#[test]
fn default_pause_blocks_decisions_until_explicit_resume() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    assert!(agent.begin(0).is_none());
    assert_eq!(agent.view(0).phase, AgentPhase::Paused);
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    assert_eq!(work.request.events.len(), 1);
    assert!(agent.begin(0).is_none());
    assert_eq!(agent.view(0).phase, AgentPhase::Deciding);
}

#[test]
fn validates_reply_ids_empty_text_topic_and_silent_decisions_atomically() {
    for decision in [
        answer(&["unknown"]),
        answer(&["one", "one"]),
        meowlive_application::ports::llm::AgentDecision {
            text: Some(" ".into()),
            ..answer(&["one"])
        },
        meowlive_application::ports::llm::AgentDecision {
            text: Some("猫".repeat(501)),
            ..answer(&["one"])
        },
        meowlive_application::ports::llm::AgentDecision {
            topic: Some("猫".repeat(201)),
            ..answer(&["one"])
        },
        meowlive_application::ports::llm::AgentDecision {
            text: None,
            ..answer(&["one"])
        },
        answer(&[]),
    ] {
        let mut agent = session();
        agent.submit(chat("one", 0), 0).unwrap();
        agent.set_paused(false, 0);
        let work = agent.begin(0).unwrap();
        assert!(
            agent
                .resolve(work.id, decision, "speech".into(), 0)
                .is_err()
        );
        assert!(agent.view(0).current_speech_id.is_none());
        assert!(agent.view(0).settings.topic.is_empty());
    }
}

#[test]
fn skip_and_model_failure_each_consume_cooldown() {
    for fail in [false, true] {
        let mut agent = session();
        agent.submit(chat("one", 0), 0).unwrap();
        agent.set_paused(false, 0);
        let work = agent.begin(0).unwrap();
        if fail {
            agent.fail(work.id, "provider unavailable".into(), 10);
        } else {
            assert!(
                agent
                    .resolve(work.id, ignore(), "unused".into(), 10)
                    .unwrap()
                    .is_none()
            );
        }
        agent.submit(chat("two", 10), 10).unwrap();
        assert!(agent.begin(1009).is_none());
        assert!(agent.begin(1010).is_some());
        assert_eq!(
            agent.view(1010).events[0].status,
            if fail {
                EventStatus::Failed
            } else {
                EventStatus::Skipped
            }
        );
    }
}

#[test]
fn pause_fences_late_decision_and_retains_pending_events() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let old = agent.begin(0).unwrap();
    agent.submit(chat("two", 1), 1).unwrap();
    agent.set_paused(true, 1);
    assert!(
        agent
            .resolve(old.id, answer(&["one"]), "late".into(), 1)
            .is_err()
    );
    agent.set_paused(false, 2);
    let next = agent.begin(2).unwrap();
    assert_ne!(old.id, next.id);
    assert_eq!(next.request.events[0].id, "two");
    assert_eq!(agent.view(2).events[0].status, EventStatus::Cancelled);
}

#[test]
fn applies_validated_topic_but_late_failures_cannot_affect_new_work() {
    let mut agent = session();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let old = agent.begin(0).unwrap();
    let mut decision = ignore();
    decision.topic = Some("猫咪日常".into());
    agent.resolve(old.id, decision, "unused".into(), 0).unwrap();
    agent.submit(chat("two", 1000), 1000).unwrap();
    let next = agent.begin(1000).unwrap();
    assert_eq!(next.request.topic, "猫咪日常");
    agent.fail(old.id, "late failure".into(), 1000);
    assert!(agent.view(1000).last_error.is_none());
    assert!(
        agent
            .resolve(next.id, answer(&["two"]), "speech".into(), 1000)
            .unwrap()
            .is_some()
    );
}
