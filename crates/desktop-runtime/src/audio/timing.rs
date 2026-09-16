//! 将设备回调的计划播放时间与真实时间关联；网络结束不等于设备完成。

use super::BackendEvent;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct DeviceClock {
    first: Option<Instant>,
    last: Option<Instant>,
    started: bool,
    completed: bool,
}

impl DeviceClock {
    pub fn submitted(
        &mut self,
        now: Instant,
        device_latency: Duration,
        frames: usize,
        sample_rate: u32,
    ) {
        if frames == 0 || sample_rate == 0 {
            return;
        }
        let first = now + device_latency;
        self.first.get_or_insert(first);
        self.last = Some(first + Duration::from_secs_f64(frames as f64 / f64::from(sample_rate)));
    }

    pub fn poll(
        &mut self,
        now: Instant,
        input_ended: bool,
        queue_empty: bool,
    ) -> Vec<BackendEvent> {
        let mut events = Vec::new();
        if !self.started && self.first.is_some_and(|deadline| now >= deadline) {
            self.started = true;
            events.push(BackendEvent::Started);
        }
        if !self.completed
            && self.started
            && input_ended
            && queue_empty
            && self.last.is_some_and(|deadline| now >= deadline)
        {
            self.completed = true;
            events.push(BackendEvent::Completed);
        }
        events
    }
}
