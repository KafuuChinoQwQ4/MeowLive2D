#![allow(dead_code)]
use meowlive_desktop_runtime::audio::{AudioBackend, BackendEvent};
use meowlive_protocol::audio::AudioFormat;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
pub struct Device {
    pub level: f32,
    pub events: Vec<BackendEvent>,
    pub stopped: bool,
    pub fail_operation: Option<&'static str>,
}
impl Device {
    fn result(&self, operation: &str) -> Result<(), String> {
        if self.fail_operation == Some(operation) {
            Err(format!("device {operation} failed"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Default)]
pub struct MeteredDevice(pub Rc<RefCell<Device>>);
impl AudioBackend for MeteredDevice {
    fn start(&mut self, _: AudioFormat) -> Result<(), String> {
        self.0.borrow_mut().stopped = false;
        self.0.borrow().result("start")
    }
    fn push(&mut self, _: &[i16]) -> Result<(), String> {
        self.0.borrow().result("push")
    }
    fn finish(&mut self) -> Result<(), String> {
        self.0.borrow().result("finish")
    }
    fn stop(&mut self) {
        self.0.borrow_mut().stopped = true;
    }
    fn poll(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.0.borrow_mut().events)
    }
    fn output_level(&mut self) -> f32 {
        self.0.borrow().level
    }
}

pub fn format() -> AudioFormat {
    AudioFormat {
        sample_rate: 24000,
        channels: 1,
    }
}
