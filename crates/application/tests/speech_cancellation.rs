mod support;

use meowlive_application::speech::SpeechQueueError;
use meowlive_domain::speech::{SpeechReceiptStatus, SpeechStatus};
use support::{connected_queue, dispatch_next};

#[test]
fn stop_increments_generation_and_cancels_active_and_waiting_tasks() {
    let mut queue = connected_queue(3, 4);
    queue.enqueue("active", "一", "default").unwrap();
    queue.enqueue("waiting", "二", "default").unwrap();
    let (id, generation) = dispatch_next(&mut queue);
    queue.apply_receipt(&id, generation, SpeechReceiptStatus::Started, None);
    assert_eq!(queue.stop(), generation + 1);
    assert!(
        queue
            .tasks()
            .all(|task| task.status == SpeechStatus::Cancelled)
    );
    assert!(queue.is_connected());
    let fresh = queue.enqueue("fresh", "三", "default").unwrap();
    assert_eq!(fresh.generation, generation + 1);
}

#[test]
fn stopped_synthesis_cannot_become_ready_or_fail_in_a_new_generation() {
    let mut queue = connected_queue(1, 4);
    queue.enqueue("old", "一", "default").unwrap();
    let old = queue.claim_next().unwrap();
    queue.stop();
    queue.enqueue("new", "二", "default").unwrap();
    assert!(!queue.mark_ready(&old.id, old.generation));
    assert!(!queue.fail(&old.id, old.generation, "late TTS error"));
    assert_eq!(queue.get("old").unwrap().status, SpeechStatus::Cancelled);
    assert_eq!(queue.claim_next().unwrap().id, "new");
}

#[test]
fn late_receipts_cannot_replace_cancellation_or_finish_the_new_task() {
    let mut queue = connected_queue(2, 4);
    queue.enqueue("old", "一", "default").unwrap();
    let (id, generation) = dispatch_next(&mut queue);
    queue.stop();
    queue.enqueue("new", "二", "default").unwrap();
    let new = queue.claim_next().unwrap();
    for status in [
        SpeechReceiptStatus::Started,
        SpeechReceiptStatus::Completed,
        SpeechReceiptStatus::Failed,
        SpeechReceiptStatus::Cancelled,
    ] {
        assert!(!queue.apply_receipt(&id, generation, status, None));
        assert!(!queue.apply_receipt(&new.id, generation, status, None));
    }
    assert_eq!(queue.get(&id).unwrap().status, SpeechStatus::Cancelled);
    assert_eq!(
        queue.get(&new.id).unwrap().status,
        SpeechStatus::Synthesizing
    );
}

#[test]
fn disconnect_marks_dispatched_tasks_unknown_and_cancels_waiting_tasks() {
    let mut queue = connected_queue(2, 4);
    queue.enqueue("sent", "一", "default").unwrap();
    queue.enqueue("waiting", "二", "default").unwrap();
    let (id, generation) = dispatch_next(&mut queue);
    assert_eq!(queue.disconnect(), generation + 1);
    assert_eq!(queue.get(&id).unwrap().status, SpeechStatus::Unknown);
    assert_eq!(
        queue.get("waiting").unwrap().status,
        SpeechStatus::Cancelled
    );
    assert!(!queue.is_connected());
    assert_eq!(
        queue.enqueue("new", "三", "default"),
        Err(SpeechQueueError::Disconnected)
    );
    assert!(queue.claim_next().is_none());
    assert!(!queue.apply_receipt(&id, generation, SpeechReceiptStatus::Completed, None));
    queue.set_connected(true);
    assert_eq!(queue.generation(), generation + 1);
    assert!(queue.claim_next().is_none());
    assert!(queue.enqueue("new", "三", "default").is_ok());
}

#[test]
fn disconnect_cancels_synthesizing_and_ready_but_undispatched_tasks() {
    for ready in [false, true] {
        let mut queue = connected_queue(1, 4);
        queue.enqueue("first", "一", "default").unwrap();
        let task = queue.claim_next().unwrap();
        if ready {
            queue.mark_ready(&task.id, task.generation);
        }
        queue.set_connected(false);
        assert_eq!(queue.get("first").unwrap().status, SpeechStatus::Cancelled);
    }
}

#[test]
fn repeated_disconnect_does_not_advance_the_generation_twice() {
    let mut queue = connected_queue(1, 4);
    let generation = queue.disconnect();
    assert_eq!(queue.disconnect(), generation);
    assert_eq!(queue.set_connected(false), generation);
}
