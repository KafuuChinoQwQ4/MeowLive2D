//! 分句、语音合成任务、并发限制和生成取消；具体引擎格式由 speech port 隔离。

use std::{collections::VecDeque, fmt};

use meowlive_domain::{
    speech::{SpeechReceiptStatus, SpeechStatus, SpeechTask, SpeechText, SpeechValidationError},
    voice::VoiceId,
};

/// A single-executor FIFO. The caller must supply session-unique task IDs.
///
/// Capacity includes the current task. Terminal snapshots consume only the
/// separately bounded history. No transport or synthesis work runs under this
/// object; callers claim work and later submit fenced results.
#[derive(Debug)]
pub struct SpeechQueue {
    capacity: usize,
    history_limit: usize,
    connected: bool,
    generation: u32,
    tasks: VecDeque<SpeechTask>,
    dispatched: Option<String>,
}

impl SpeechQueue {
    pub fn new(capacity: usize, history_limit: usize) -> Self {
        Self {
            capacity,
            history_limit,
            connected: false,
            generation: 0,
            tasks: VecDeque::new(),
            dispatched: None,
        }
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn set_connected(&mut self, connected: bool) -> u32 {
        if !connected {
            return self.disconnect();
        }
        self.connected = true;
        self.generation
    }

    pub fn enqueue(
        &mut self,
        id: impl Into<String>,
        text: impl Into<String>,
        voice_id: impl Into<String>,
    ) -> Result<SpeechTask, SpeechQueueError> {
        let text = SpeechText::new(text).map_err(SpeechQueueError::InvalidInput)?;
        self.enqueue_validated(id, text, voice_id)
    }

    pub fn enqueue_broadcast(
        &mut self,
        id: impl Into<String>,
        text: impl Into<String>,
        voice_id: impl Into<String>,
    ) -> Result<SpeechTask, SpeechQueueError> {
        let text = SpeechText::broadcast(text).map_err(SpeechQueueError::InvalidInput)?;
        self.enqueue_validated(id, text, voice_id)
    }

    fn enqueue_validated(
        &mut self,
        id: impl Into<String>,
        text: SpeechText,
        voice_id: impl Into<String>,
    ) -> Result<SpeechTask, SpeechQueueError> {
        if !self.connected {
            return Err(SpeechQueueError::Disconnected);
        }
        let voice_id = VoiceId::new(voice_id).map_err(SpeechQueueError::InvalidInput)?;
        let id = id.into();
        if self.get(&id).is_some() {
            return Err(SpeechQueueError::DuplicateId);
        }
        if self
            .tasks
            .iter()
            .filter(|task| !task.status.is_terminal())
            .count()
            >= self.capacity
        {
            return Err(SpeechQueueError::Full);
        }
        let task = SpeechTask {
            id,
            generation: self.generation,
            text,
            voice_id,
            status: SpeechStatus::Queued,
            error: None,
        };
        self.tasks.push_back(task.clone());
        Ok(task)
    }

    pub fn tasks(&self) -> impl Iterator<Item = &SpeechTask> {
        self.tasks.iter()
    }

    pub fn get(&self, id: &str) -> Option<&SpeechTask> {
        self.tasks.iter().find(|task| task.id == id)
    }

    pub fn claim_next(&mut self) -> Option<SpeechTask> {
        if !self.connected
            || self.tasks.iter().any(|task| {
                matches!(
                    task.status,
                    SpeechStatus::Synthesizing | SpeechStatus::Ready | SpeechStatus::Playing
                )
            })
        {
            return None;
        }
        let task = self
            .tasks
            .iter_mut()
            .find(|task| task.status == SpeechStatus::Queued)?;
        task.status = SpeechStatus::Synthesizing;
        Some(task.clone())
    }

    pub fn next_for_synthesis(&mut self) -> Option<SpeechTask> {
        self.claim_next()
    }

    pub fn mark_ready(&mut self, id: &str, generation: u32) -> bool {
        let Some(task) = self.current_task_mut(id, generation) else {
            return false;
        };
        if task.status != SpeechStatus::Synthesizing {
            return false;
        }
        task.status = SpeechStatus::Ready;
        true
    }

    /// Mark immediately before sending the command so any partial send followed
    /// by a disconnect is treated as an uncertain execution, never replayed.
    pub fn mark_dispatched(&mut self, id: &str, generation: u32) -> bool {
        if self.dispatched.is_some() {
            return false;
        }
        let Some(task) = self.current_task_mut(id, generation) else {
            return false;
        };
        if task.status != SpeechStatus::Ready {
            return false;
        }
        self.dispatched = Some(id.to_owned());
        true
    }

    pub fn apply_receipt(
        &mut self,
        id: &str,
        generation: u32,
        status: SpeechReceiptStatus,
        error: Option<String>,
    ) -> bool {
        if self.dispatched.as_deref() != Some(id) {
            return false;
        }
        let Some(task) = self.current_task_mut(id, generation) else {
            return false;
        };
        let next = match (task.status, status) {
            (SpeechStatus::Ready, SpeechReceiptStatus::Started) => SpeechStatus::Playing,
            (SpeechStatus::Playing, SpeechReceiptStatus::Completed) => SpeechStatus::Completed,
            (SpeechStatus::Ready | SpeechStatus::Playing, SpeechReceiptStatus::Cancelled) => {
                SpeechStatus::Cancelled
            }
            (SpeechStatus::Ready | SpeechStatus::Playing, SpeechReceiptStatus::Failed) => {
                SpeechStatus::Failed
            }
            _ => return false,
        };
        task.status = next;
        task.error = if next == SpeechStatus::Failed {
            error
        } else {
            None
        };
        if next.is_terminal() {
            self.dispatched = None;
            self.prune_history();
        }
        true
    }

    pub fn fail(&mut self, id: &str, generation: u32, error: impl Into<String>) -> bool {
        let Some(task) = self.current_task_mut(id, generation) else {
            return false;
        };
        if task.status.is_terminal() {
            return false;
        }
        task.status = SpeechStatus::Failed;
        task.error = Some(error.into());
        if self.dispatched.as_deref() == Some(id) {
            self.dispatched = None;
        }
        self.prune_history();
        true
    }

    pub fn stop(&mut self) -> u32 {
        self.advance_generation();
        for task in &mut self.tasks {
            if !task.status.is_terminal() {
                task.status = SpeechStatus::Cancelled;
                task.error = None;
            }
        }
        self.dispatched = None;
        self.prune_history();
        self.generation
    }

    pub fn disconnect(&mut self) -> u32 {
        if !self.connected {
            return self.generation;
        }
        self.connected = false;
        self.advance_generation();
        for task in &mut self.tasks {
            if !task.status.is_terminal() {
                task.status = if self.dispatched.as_deref() == Some(task.id.as_str()) {
                    SpeechStatus::Unknown
                } else {
                    SpeechStatus::Cancelled
                };
                task.error = None;
            }
        }
        self.dispatched = None;
        self.prune_history();
        self.generation
    }

    fn current_task_mut(&mut self, id: &str, generation: u32) -> Option<&mut SpeechTask> {
        if !self.connected || generation != self.generation {
            return None;
        }
        self.tasks
            .iter_mut()
            .find(|task| task.id == id && task.generation == generation)
    }

    fn advance_generation(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("speech generation exhausted");
    }

    fn prune_history(&mut self) {
        let mut excess = self
            .tasks
            .iter()
            .filter(|task| task.status.is_terminal())
            .count()
            .saturating_sub(self.history_limit);
        self.tasks.retain(|task| {
            if excess > 0 && task.status.is_terminal() {
                excess -= 1;
                false
            } else {
                true
            }
        });
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeechQueueError {
    Disconnected,
    Full,
    DuplicateId,
    InvalidInput(SpeechValidationError),
}

impl fmt::Display for SpeechQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => formatter.write_str("desktop executor is not connected"),
            Self::Full => formatter.write_str("speech queue is full"),
            Self::DuplicateId => formatter.write_str("speech ID is already in use"),
            Self::InvalidInput(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SpeechQueueError {}
