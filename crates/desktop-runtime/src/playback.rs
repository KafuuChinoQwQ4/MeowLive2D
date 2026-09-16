//! 管理本地播放队列、代次校验、取消与执行回执；不把网络操作放入音频回调。

use crate::audio::{AudioBackend, BackendEvent};
use meowlive_protocol::{
    audio::{AudioChunk, MAX_AUDIO_SAMPLES},
    control::ServerCommand,
    execution::{ExecutionReceipt, ExecutionStatus},
};
use std::collections::VecDeque;

struct Active {
    id: String,
    sequence: u32,
    channels: u16,
    ended: bool,
    started: bool,
    received_samples: usize,
}

pub struct Playback<B: AudioBackend> {
    backend: B,
    generation: u32,
    active: Option<Active>,
    pending: VecDeque<AudioChunk>,
    pending_samples: usize,
    maximum: usize,
    retired: VecDeque<String>,
}

impl<B: AudioBackend> Playback<B> {
    pub fn new(backend: B, generation: u32, maximum: usize) -> Self {
        Self {
            backend,
            generation,
            active: None,
            pending: VecDeque::new(),
            pending_samples: 0,
            maximum,
            retired: VecDeque::new(),
        }
    }

    pub fn command(&mut self, command: ServerCommand) -> Result<Vec<ExecutionReceipt>, String> {
        match command {
            ServerCommand::Resource { .. } => {
                Err("resource command must use the resource executor".into())
            }
            ServerCommand::Hello { .. } => Err("duplicate hello on paired connection".into()),
            ServerCommand::Stop { generation } => {
                if generation <= self.generation {
                    return Ok(Vec::new());
                }
                let receipts = self.terminate(ExecutionStatus::Cancelled, None);
                self.generation = generation;
                self.pending.retain(|chunk| chunk.generation >= generation);
                self.pending_samples = self.pending.iter().map(|chunk| chunk.samples.len()).sum();
                self.retired.clear();
                Ok(receipts)
            }
            ServerCommand::Speak {
                utterance_id,
                generation,
                format,
            } => {
                if generation < self.generation {
                    return Ok(Vec::new());
                }
                if generation != self.generation
                    || self.active.is_some()
                    || self.retired.contains(&utterance_id)
                {
                    return Err("unexpected generation or duplicate/concurrent speak".into());
                }
                format.validate()?;
                self.active = Some(Active {
                    id: utterance_id.clone(),
                    sequence: 0,
                    channels: format.channels,
                    ended: false,
                    started: false,
                    received_samples: 0,
                });
                if let Err(error) = self.backend.start(format) {
                    return Ok(self.terminate(ExecutionStatus::Failed, Some(error)));
                }
                let mut remaining = VecDeque::new();
                let mut selected = Vec::new();
                while let Some(chunk) = self.pending.pop_front() {
                    if chunk.utterance_id == utterance_id && chunk.generation == generation {
                        self.pending_samples -= chunk.samples.len();
                        selected.push(chunk);
                    } else {
                        remaining.push_back(chunk);
                    }
                }
                self.pending = remaining;
                let mut receipts = Vec::new();
                for chunk in selected {
                    receipts.extend(self.chunk(chunk)?);
                }
                Ok(receipts)
            }
        }
    }

    pub fn chunk(&mut self, chunk: AudioChunk) -> Result<Vec<ExecutionReceipt>, String> {
        if chunk.generation < self.generation
            || (chunk.generation == self.generation && self.retired.contains(&chunk.utterance_id))
        {
            return Ok(Vec::new());
        }
        if chunk.samples.len() > MAX_AUDIO_SAMPLES {
            return Err("oversized audio payload".into());
        }
        if !self.active.as_ref().is_some_and(|active| {
            active.id == chunk.utterance_id && chunk.generation == self.generation
        }) {
            if self.pending_samples + chunk.samples.len() > self.maximum || self.pending.len() >= 64
            {
                return Err("audio waiting for control exceeds buffer capacity".into());
            }
            self.pending_samples += chunk.samples.len();
            self.pending.push_back(chunk);
            return Ok(Vec::new());
        }
        let active = self.active.as_mut().expect("active checked above");
        if active.ended
            || chunk.sequence != active.sequence
            || chunk.samples.len() % usize::from(active.channels) != 0
        {
            return Err("audio sequence, end marker or channel alignment is invalid".into());
        }
        active.sequence = active
            .sequence
            .checked_add(1)
            .ok_or("audio sequence exhausted")?;
        active.ended = chunk.end;
        active.received_samples = active.received_samples.saturating_add(chunk.samples.len());
        if chunk.end && active.received_samples == 0 {
            return Ok(self.terminate(
                ExecutionStatus::Failed,
                Some("utterance contains no PCM samples".into()),
            ));
        }
        if let Err(error) = self.backend.push(&chunk.samples) {
            return Ok(self.terminate(ExecutionStatus::Failed, Some(error)));
        }
        if chunk.end {
            if let Err(error) = self.backend.finish() {
                return Ok(self.terminate(ExecutionStatus::Failed, Some(error)));
            }
        }
        Ok(Vec::new())
    }

    pub fn poll(&mut self) -> Vec<ExecutionReceipt> {
        let mut receipts = Vec::new();
        for event in self.backend.poll() {
            let Some(active) = self.active.as_mut() else {
                continue;
            };
            match event {
                BackendEvent::Started if !active.started => {
                    active.started = true;
                    receipts.push(ExecutionReceipt {
                        utterance_id: active.id.clone(),
                        generation: self.generation,
                        status: ExecutionStatus::Started,
                        error: None,
                    });
                }
                BackendEvent::Started => {}
                BackendEvent::Completed if active.ended => {
                    receipts.extend(self.terminate(ExecutionStatus::Completed, None))
                }
                BackendEvent::Completed => receipts.extend(self.terminate(
                    ExecutionStatus::Failed,
                    Some("device completed before audio end".into()),
                )),
                BackendEvent::Failed(error) => {
                    receipts.extend(self.terminate(ExecutionStatus::Failed, Some(error)))
                }
            }
        }
        receipts
    }

    fn terminate(
        &mut self,
        status: ExecutionStatus,
        error: Option<String>,
    ) -> Vec<ExecutionReceipt> {
        self.backend.stop();
        let Some(active) = self.active.take() else {
            return Vec::new();
        };
        self.retired.push_back(active.id.clone());
        if self.retired.len() > 128 {
            self.retired.pop_front();
        }
        vec![ExecutionReceipt {
            utterance_id: active.id,
            generation: self.generation,
            status,
            error,
        }]
    }

    pub fn disconnect(&mut self) {
        self.backend.stop();
        self.active = None;
        self.pending.clear();
        self.pending_samples = 0;
    }
}

impl<B: AudioBackend> Drop for Playback<B> {
    fn drop(&mut self) {
        self.disconnect();
    }
}
