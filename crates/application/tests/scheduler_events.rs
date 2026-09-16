mod agent_support;
use agent_support::{chat, gift, session};
use meowlive_application::agent::{
    AgentLimits, AgentSession, AgentSettings, EventStatus, SubmitOutcome,
};

#[test]
fn duplicate_replay_is_ignored_without_a_second_record() {
    let mut agent = session();
    assert_eq!(
        agent.submit(chat("one", 0), 0).unwrap(),
        SubmitOutcome::Accepted
    );
    assert_eq!(
        agent.submit(chat("one", 0), 0).unwrap(),
        SubmitOutcome::Duplicate
    );
    assert_eq!(agent.view(0).events.len(), 1);
}

#[test]
fn expired_events_are_visible_but_never_selected() {
    let mut agent = session();
    agent.submit(chat("old", 0), 60_000).unwrap();
    agent.set_paused(false, 60_000);
    assert!(agent.begin(60_000).is_none());
    assert_eq!(agent.view(60_000).events[0].status, EventStatus::Expired);
    agent.submit(chat("pending", 60_000), 60_000).unwrap();
    assert_eq!(agent.view(120_000).events[1].status, EventStatus::Expired);
}

#[test]
fn rejects_future_timestamp_and_pending_overflow_without_reserving_id() {
    let mut agent = AgentSession::new(
        AgentSettings::default(),
        AgentLimits {
            pending_capacity: 1,
            ..AgentLimits::default()
        },
    )
    .unwrap();
    assert!(agent.submit(chat("future", 5001), 0).is_err());
    agent.submit(chat("one", 0), 0).unwrap();
    assert!(
        agent
            .submit(chat("two", 0), 0)
            .unwrap_err()
            .contains("capacity")
    );
    agent.stop(0);
    assert_eq!(
        agent.submit(chat("two", 0), 0).unwrap(),
        SubmitOutcome::Accepted
    );
}

#[test]
fn gift_batches_take_priority_and_merge_group_selection_is_atomic() {
    let mut agent = session();
    agent.submit(chat("chat", 0), 0).unwrap();
    agent.submit(gift("gift-1", 100, 2), 100).unwrap();
    agent.submit(gift("gift-2", 200, 3), 200).unwrap();
    agent.set_paused(false, 200);
    let work = agent.begin(200).unwrap();
    assert_eq!(
        work.request
            .events
            .iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>(),
        vec!["gift-1", "gift-2", "chat"]
    );
    agent
        .resolve(
            work.id,
            agent_support::answer(&["gift-1", "gift-2"]),
            "speech".into(),
            200,
        )
        .unwrap();
    let view = agent.view(200);
    assert_eq!(view.events[0].status, EventStatus::Skipped);
    assert_eq!(view.events[1].status, EventStatus::Queued);
    assert_eq!(view.events[2].status, EventStatus::Queued);
}

#[test]
fn gift_groups_do_not_cross_viewers_or_merge_window() {
    let mut agent = session();
    agent.submit(gift("first", 0, 1), 0).unwrap();
    agent.submit(gift("later", 3001, 1), 3001).unwrap();
    let mut other = gift("other", 3001, 1);
    other.viewer = "别人".into();
    agent.submit(other, 3001).unwrap();
    agent.set_paused(false, 3001);
    let work = agent.begin(3001).unwrap();
    agent
        .resolve(
            work.id,
            agent_support::answer(&["first"]),
            "speech".into(),
            3001,
        )
        .unwrap();
    assert_eq!(
        agent
            .view(3001)
            .events
            .iter()
            .map(|record| record.status)
            .collect::<Vec<_>>(),
        vec![
            EventStatus::Queued,
            EventStatus::Skipped,
            EventStatus::Skipped
        ]
    );
}

#[test]
fn bounds_candidate_batch_and_keeps_unselected_events_pending() {
    let mut agent = session();
    for number in 0..12 {
        agent.submit(chat(&number.to_string(), 0), 0).unwrap();
    }
    agent.set_paused(false, 0);
    let work = agent.begin(0).unwrap();
    assert_eq!(work.request.events.len(), 8);
    assert_eq!(
        agent
            .view(0)
            .events
            .iter()
            .filter(|record| record.status == EventStatus::Pending)
            .count(),
        4
    );
}

#[test]
fn decision_cannot_acknowledge_only_part_of_a_merged_gift_group() {
    let mut agent = session();
    agent.submit(gift("gift-1", 0, 1), 0).unwrap();
    agent.submit(gift("gift-2", 1, 1), 1).unwrap();
    agent.set_paused(false, 1);
    let work = agent.begin(1).unwrap();
    assert!(
        agent
            .resolve(
                work.id,
                agent_support::answer(&["gift-1"]),
                "speech".into(),
                1
            )
            .is_err()
    );
    assert!(agent.view(1).current_speech_id.is_none());
}
