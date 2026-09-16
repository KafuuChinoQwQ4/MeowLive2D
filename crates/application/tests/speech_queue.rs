mod support;

use meowlive_application::speech::{SpeechQueue, SpeechQueueError};
use meowlive_domain::speech::{SpeechStatus, SpeechValidationError};
use support::{complete_next, connected_queue};

#[test]
fn rejects_submission_until_the_executor_is_connected() {
    let mut queue = SpeechQueue::new(2, 4);
    assert_eq!(
        queue.enqueue("first", "你好", "default"),
        Err(SpeechQueueError::Disconnected)
    );
    assert!(queue.claim_next().is_none());
    assert_eq!(queue.tasks().count(), 0);
}

#[test]
fn validates_submission_before_it_consumes_capacity() {
    let mut queue = connected_queue(1, 4);
    assert_eq!(
        queue.enqueue("bad", " ", "default"),
        Err(SpeechQueueError::InvalidInput(
            SpeechValidationError::EmptyText
        ))
    );
    assert_eq!(
        queue.enqueue("bad", "你好", "../voice"),
        Err(SpeechQueueError::InvalidInput(
            SpeechValidationError::InvalidVoiceId
        ))
    );
    let task = queue.enqueue("good", "  你好  ", "default").unwrap();
    assert_eq!(task.text.as_str(), "你好");
    assert_eq!(task.voice_id.as_str(), "default");
    assert_eq!(task.status, SpeechStatus::Queued);
}

#[test]
fn claims_in_fifo_order_and_waits_until_the_active_task_is_terminal() {
    let mut queue = connected_queue(3, 4);
    queue.enqueue("first", "一", "default").unwrap();
    queue.enqueue("second", "二", "default").unwrap();
    let task = queue.claim_next().unwrap();
    assert_eq!(task.id, "first");
    assert_eq!(task.status, SpeechStatus::Synthesizing);
    assert!(queue.claim_next().is_none());
    assert!(queue.mark_ready(&task.id, task.generation));
    assert!(queue.claim_next().is_none());
    assert!(queue.fail(&task.id, task.generation, "dispatch failed"));
    assert_eq!(queue.claim_next().unwrap().id, "second");
}

#[test]
fn capacity_counts_waiting_and_active_tasks_but_excludes_history() {
    let mut queue = connected_queue(2, 4);
    queue.enqueue("first", "一", "default").unwrap();
    queue.enqueue("second", "二", "default").unwrap();
    let task = queue.claim_next().unwrap();
    assert_eq!(
        queue.enqueue("third", "三", "default"),
        Err(SpeechQueueError::Full)
    );
    queue.fail(&task.id, task.generation, "TTS unavailable");
    assert!(queue.enqueue("third", "三", "default").is_ok());
    assert_eq!(queue.tasks().count(), 3);
}

#[test]
fn zero_capacity_rejects_every_submission() {
    let mut queue = connected_queue(0, 4);
    assert_eq!(
        queue.enqueue("first", "一", "default"),
        Err(SpeechQueueError::Full)
    );
}

#[test]
fn duplicate_retained_ids_cannot_target_two_tasks() {
    let mut queue = connected_queue(3, 4);
    queue.enqueue("same", "一", "default").unwrap();
    assert_eq!(
        queue.enqueue("same", "二", "default"),
        Err(SpeechQueueError::DuplicateId)
    );
    complete_next(&mut queue);
    assert_eq!(
        queue.enqueue("same", "三", "default"),
        Err(SpeechQueueError::DuplicateId)
    );
}

#[test]
fn keeps_only_the_latest_terminal_history_without_dropping_waiting_tasks() {
    let mut queue = connected_queue(3, 2);
    for id in ["first", "second", "third"] {
        queue.enqueue(id, "你好", "default").unwrap();
        complete_next(&mut queue);
    }
    queue.enqueue("waiting", "你好", "default").unwrap();
    let ids: Vec<_> = queue.tasks().map(|task| task.id.as_str()).collect();
    assert_eq!(ids, ["second", "third", "waiting"]);
    assert_eq!(queue.get("waiting").unwrap().status, SpeechStatus::Queued);
    assert!(queue.get("first").is_none());
}

#[test]
fn zero_history_keeps_active_tasks_and_removes_them_when_terminal() {
    let mut queue = connected_queue(1, 0);
    queue.enqueue("first", "你好", "default").unwrap();
    assert_eq!(queue.tasks().count(), 1);
    complete_next(&mut queue);
    assert_eq!(queue.tasks().count(), 0);
    assert!(queue.enqueue("second", "你好", "default").is_ok());
}
