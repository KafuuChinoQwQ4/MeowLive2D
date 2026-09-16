mod support;

use meowlive_domain::speech::{SpeechReceiptStatus, SpeechStatus};
use support::{connected_queue, dispatch_next};

#[test]
fn completion_requires_a_started_receipt_for_the_dispatched_task() {
    let mut queue = connected_queue(2, 4);
    queue.enqueue("first", "一", "default").unwrap();
    queue.enqueue("second", "二", "default").unwrap();
    let (id, generation) = dispatch_next(&mut queue);
    assert!(!queue.apply_receipt(&id, generation, SpeechReceiptStatus::Completed, None));
    assert_eq!(queue.get(&id).unwrap().status, SpeechStatus::Ready);
    assert!(queue.apply_receipt(&id, generation, SpeechReceiptStatus::Started, None));
    assert_eq!(queue.get(&id).unwrap().status, SpeechStatus::Playing);
    assert!(queue.claim_next().is_none());
    assert!(queue.apply_receipt(&id, generation, SpeechReceiptStatus::Completed, None));
    assert_eq!(queue.get(&id).unwrap().status, SpeechStatus::Completed);
    assert_eq!(queue.claim_next().unwrap().id, "second");
}

#[test]
fn queued_synthesizing_and_undispatched_ready_tasks_reject_receipts() {
    let mut queue = connected_queue(1, 4);
    let task = queue.enqueue("first", "一", "default").unwrap();
    for stage in 0..3 {
        if stage == 1 {
            queue.claim_next();
        } else if stage == 2 {
            queue.mark_ready(&task.id, task.generation);
        }
        for status in [
            SpeechReceiptStatus::Started,
            SpeechReceiptStatus::Completed,
            SpeechReceiptStatus::Failed,
            SpeechReceiptStatus::Cancelled,
        ] {
            assert!(!queue.apply_receipt(&task.id, task.generation, status, None));
        }
    }
}

#[test]
fn invalid_ids_generations_and_duplicate_receipts_cannot_change_state() {
    let mut queue = connected_queue(1, 4);
    queue.enqueue("first", "一", "default").unwrap();
    let (id, generation) = dispatch_next(&mut queue);
    assert!(!queue.apply_receipt("missing", generation, SpeechReceiptStatus::Started, None));
    assert!(!queue.apply_receipt(&id, generation + 1, SpeechReceiptStatus::Started, None));
    assert!(queue.apply_receipt(&id, generation, SpeechReceiptStatus::Started, None));
    assert!(!queue.apply_receipt(&id, generation, SpeechReceiptStatus::Started, None));
    assert!(queue.apply_receipt(&id, generation, SpeechReceiptStatus::Completed, None));
    assert!(!queue.apply_receipt(
        &id,
        generation,
        SpeechReceiptStatus::Failed,
        Some("late failure".into())
    ));
    assert!(!queue.fail(&id, generation, "late service failure"));
    assert_eq!(queue.get(&id).unwrap().status, SpeechStatus::Completed);
    assert!(queue.get(&id).unwrap().error.is_none());
}

#[test]
fn executor_failure_before_started_releases_the_queue_and_keeps_error() {
    let mut queue = connected_queue(2, 4);
    queue.enqueue("first", "一", "default").unwrap();
    queue.enqueue("second", "二", "default").unwrap();
    let (id, generation) = dispatch_next(&mut queue);
    assert!(queue.apply_receipt(
        &id,
        generation,
        SpeechReceiptStatus::Failed,
        Some("audio device unavailable".into())
    ));
    let task = queue.get(&id).unwrap();
    assert_eq!(task.status, SpeechStatus::Failed);
    assert_eq!(task.error.as_deref(), Some("audio device unavailable"));
    assert_eq!(queue.claim_next().unwrap().id, "second");
}

#[test]
fn executor_cancellation_is_terminal_and_does_not_advance_generation() {
    let mut queue = connected_queue(1, 4);
    queue.enqueue("first", "一", "default").unwrap();
    let (id, generation) = dispatch_next(&mut queue);
    assert!(queue.apply_receipt(&id, generation, SpeechReceiptStatus::Cancelled, None));
    assert_eq!(queue.get(&id).unwrap().status, SpeechStatus::Cancelled);
    assert_eq!(queue.generation(), generation);
}

#[test]
fn ready_and_dispatch_transitions_require_the_matching_prior_state() {
    let mut queue = connected_queue(1, 4);
    let task = queue.enqueue("first", "一", "default").unwrap();
    assert!(!queue.mark_ready(&task.id, task.generation));
    assert!(!queue.mark_dispatched(&task.id, task.generation));
    queue.claim_next();
    assert!(!queue.mark_dispatched(&task.id, task.generation));
    assert!(!queue.mark_ready(&task.id, task.generation + 1));
    assert!(queue.mark_ready(&task.id, task.generation));
    assert!(!queue.mark_ready(&task.id, task.generation));
    assert!(!queue.mark_dispatched("missing", task.generation));
    assert!(queue.mark_dispatched(&task.id, task.generation));
    assert!(!queue.mark_dispatched(&task.id, task.generation));
}
