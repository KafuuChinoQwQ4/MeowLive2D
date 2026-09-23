//! 播放结果是会话记忆的提交点；取消、失败和未知状态不会重复发言。
use super::{AgentSession, EventStatus};
use meowlive_domain::speech::{SpeechStatus, SpeechTask};

impl AgentSession {
    pub fn speech_cancelled(&mut self, speech_id: &str, now_ms: u64) {
        if self
            .current
            .as_ref()
            .is_none_or(|speech| speech.id != speech_id)
        {
            return;
        }
        let speech = self
            .current
            .take()
            .expect("matched active speech remains present");
        self.scheduler.update(
            &speech.event_ids,
            EventStatus::Cancelled,
            Some(speech_id),
            None,
        );
        self.cooldown(now_ms);
    }

    pub fn speech_failed(&mut self, speech_id: &str, message: String, now_ms: u64) {
        if self
            .current
            .as_ref()
            .is_none_or(|speech| speech.id != speech_id)
        {
            return;
        }
        let speech = self
            .current
            .take()
            .expect("matched active speech remains present");
        let message: String = message.chars().take(1000).collect();
        self.scheduler.update(
            &speech.event_ids,
            EventStatus::Failed,
            Some(speech_id),
            Some(&message),
        );
        self.last_error = Some(message);
        self.cooldown(now_ms);
    }

    pub fn sync_speech(&mut self, task: &SpeechTask, now_ms: u64) {
        let Some(current) = self.current.as_ref().filter(|speech| speech.id == task.id) else {
            return;
        };
        let status = match task.status {
            SpeechStatus::Queued => EventStatus::Queued,
            SpeechStatus::Synthesizing => EventStatus::Synthesizing,
            SpeechStatus::Ready => EventStatus::Ready,
            SpeechStatus::Playing => EventStatus::Playing,
            SpeechStatus::Completed => EventStatus::Completed,
            SpeechStatus::Cancelled => EventStatus::Cancelled,
            SpeechStatus::Failed => EventStatus::Failed,
            SpeechStatus::Unknown => EventStatus::Unknown,
        };
        self.scheduler.update(
            &current.event_ids,
            status,
            Some(&task.id),
            task.error.as_deref(),
        );
        if !task.status.is_terminal() {
            return;
        }
        let speech = self
            .current
            .take()
            .expect("matched active speech remains present");
        if task.status == SpeechStatus::Completed {
            self.scheduler
                .completed(&speech.events, &speech.turn.assistant, now_ms);
            self.completed.push(super::CompletedInteraction {
                speech_id: speech.id,
                events: speech.events,
                assistant: speech.turn.assistant.clone(),
            });
            self.history.push_back(speech.turn);
            while self.history.len() > self.scheduler.limits.conversation_limit {
                self.history.pop_front();
            }
        }
        if let Some(message) = &task.error {
            self.last_error = Some(message.chars().take(1000).collect());
        }
        self.cooldown(now_ms);
    }
}
