mod agent_support;
use agent_support::{answer, chat, gift, ignore};
use meowlive_application::agent::{
    AgentLimits, AgentSession, AgentSettings, EventStatus, SubmitOutcome,
};

fn limited() -> AgentSession {
    AgentSession::new(
        AgentSettings {
            cooldown_ms: 1000,
            ..AgentSettings::default()
        },
        AgentLimits {
            history_limit: 1,
            dedup_capacity: 2,
            batch_size: 2,
            ..AgentLimits::default()
        },
    )
    .unwrap()
}

#[test]
fn history_pruning_preserves_dedup_and_pending_records() {
    let mut agent = limited();
    agent.submit(chat("one", 0), 0).unwrap();
    agent.submit(chat("two", 0), 0).unwrap();
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    agent
        .resolve(work.id, ignore(), "unused".into(), 0)
        .unwrap();
    assert_eq!(agent.view(0).events.len(), 1);
    assert_eq!(
        agent.submit(chat("one", 0), 0).unwrap(),
        SubmitOutcome::Duplicate
    );
    agent.submit(chat("pending-a", 1), 1).unwrap();
    agent.submit(chat("pending-b", 1), 1).unwrap();
    assert_eq!(agent.view(1).events.len(), 3);
}

#[test]
fn dedup_eviction_allows_old_terminal_ids_but_keeps_live_ids_pinned() {
    let mut agent = limited();
    for id in ["one", "two", "three"] {
        agent.submit(chat(id, 0), 0).unwrap();
    }
    assert_eq!(
        agent.submit(chat("one", 0), 0).unwrap(),
        SubmitOutcome::Duplicate
    );
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    assert_eq!(
        agent.submit(chat("one", 0), 0).unwrap(),
        SubmitOutcome::Duplicate
    );
    agent
        .resolve(work.id, answer(&["one"]), "speech".into(), 0)
        .unwrap();
    assert_eq!(
        agent.submit(chat("one", 0), 0).unwrap(),
        SubmitOutcome::Duplicate
    );
    agent.speech_failed("speech", "unavailable".into(), 1);
    assert_eq!(
        agent.submit(chat("one", 1), 1).unwrap(),
        SubmitOutcome::Accepted
    );
}

#[test]
fn global_event_ids_prevent_ambiguous_reply_to_across_sources() {
    let mut agent = limited();
    agent.submit(chat("shared", 0), 0).unwrap();
    let mut duplicate = chat("shared", 0);
    duplicate.source = "replay".into();
    assert_eq!(
        agent.submit(duplicate, 0).unwrap(),
        SubmitOutcome::Duplicate
    );
}

#[test]
fn oversized_gift_run_is_split_into_bounded_atomic_groups_without_starvation() {
    let mut agent = limited();
    for id in ["one", "two", "three"] {
        agent.submit(gift(id, 0, 1), 0).unwrap();
    }
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    assert_eq!(work.request.events.len(), 2);
    agent
        .resolve(work.id, answer(&["one", "two"]), "speech".into(), 0)
        .unwrap();
    assert_eq!(agent.view(0).events[2].status, EventStatus::Pending);
    agent.speech_failed("speech", "failed".into(), 1);
    let next = agent.begin(1001).unwrap();
    assert_eq!(next.request.events[0].id, "three");
}

#[test]
fn cloning_supports_batch_validation_without_mutating_original() {
    let mut agent = limited();
    let mut proposal = agent.clone();
    proposal.submit(chat("one", 0), 0).unwrap();
    assert!(agent.view(0).events.is_empty());
    agent = proposal;
    assert_eq!(agent.view(0).events.len(), 1);
}
