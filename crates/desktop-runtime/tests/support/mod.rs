use meowlive_desktop_runtime::audio::{AudioBackend, BackendEvent};
use meowlive_protocol::{
    audio::{AudioChunk, AudioFormat},
    control::ServerCommand,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
pub struct DeviceState {
    pub samples: Vec<i16>,
    pub ended: bool,
    pub stopped: bool,
    pub events: Vec<BackendEvent>,
}

#[derive(Clone, Default)]
pub struct ManualDevice(pub Rc<RefCell<DeviceState>>);

impl AudioBackend for ManualDevice {
    fn start(&mut self, _: AudioFormat) -> Result<(), String> {
        Ok(())
    }
    fn push(&mut self, samples: &[i16]) -> Result<(), String> {
        self.0.borrow_mut().samples.extend_from_slice(samples);
        Ok(())
    }
    fn finish(&mut self) -> Result<(), String> {
        self.0.borrow_mut().ended = true;
        Ok(())
    }
    fn stop(&mut self) {
        self.0.borrow_mut().stopped = true;
        self.0.borrow_mut().events.clear();
    }
    fn poll(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.0.borrow_mut().events)
    }
}

pub fn speak(generation: u32) -> ServerCommand {
    ServerCommand::Speak {
        utterance_id: "u1".into(),
        generation,
        format: AudioFormat {
            sample_rate: 24_000,
            channels: 1,
        },
    }
}

pub fn chunk(generation: u32, sequence: u32, end: bool) -> AudioChunk {
    AudioChunk {
        utterance_id: "u1".into(),
        generation,
        sequence,
        samples: vec![1, 2, 3],
        end,
    }
}
