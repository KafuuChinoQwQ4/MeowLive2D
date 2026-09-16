use meowlive_application::speech::SpeechQueue;
use meowlive_domain::speech::SpeechReceiptStatus;

pub fn connected_queue(capacity: usize, history_limit: usize) -> SpeechQueue {
    let mut queue = SpeechQueue::new(capacity, history_limit);
    queue.set_connected(true);
    queue
}

#[allow(dead_code)]
pub fn dispatch_next(queue: &mut SpeechQueue) -> (String, u32) {
    let task = queue.claim_next().unwrap();
    assert!(queue.mark_ready(&task.id, task.generation));
    assert!(queue.mark_dispatched(&task.id, task.generation));
    (task.id, task.generation)
}

#[allow(dead_code)]
pub fn complete_next(queue: &mut SpeechQueue) {
    let (id, generation) = dispatch_next(queue);
    assert!(queue.apply_receipt(&id, generation, SpeechReceiptStatus::Started, None));
    assert!(queue.apply_receipt(&id, generation, SpeechReceiptStatus::Completed, None));
}
