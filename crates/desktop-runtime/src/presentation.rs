//! 桌面进程的表现输出组装；VTS 生命周期独立于主服务重连和单条播报。
pub use crate::avatar::switching::MouthControl;
use crate::{
    audio::AudioBackend,
    avatar::{self, AvatarStatus, VtsConfig},
    lip_sync::{LipSyncBackend, LipSyncConfig},
};
use std::time::Duration;
use tokio::{sync::watch, task::JoinHandle};

pub struct AvatarDriver {
    levels: Option<watch::Sender<f64>>,
    status: watch::Receiver<AvatarStatus>,
    task: Option<JoinHandle<()>>,
    shutdown_timeout: Duration,
    mouth: MouthControl,
}

impl AvatarDriver {
    pub fn start(config: VtsConfig) -> Result<Self, String> {
        config.validate()?;
        let shutdown_timeout = Duration::from_millis(config.request_timeout_ms + 100);
        let (levels, receiver) = watch::channel(0.0);
        let (sender, status) = watch::channel(AvatarStatus::Disabled);
        let (mouth, parameters) = watch::channel(config.mouth_parameter.clone());
        let (applied_sender, applied) = watch::channel(None);
        let task = tokio::spawn(avatar::switching::run(
            config,
            receiver,
            parameters,
            sender,
            applied_sender,
        ));
        Ok(Self {
            levels: Some(levels),
            status,
            task: Some(task),
            shutdown_timeout,
            mouth: MouthControl {
                requested: mouth,
                applied,
            },
        })
    }

    pub fn observe<B: AudioBackend>(
        &self,
        backend: B,
        config: LipSyncConfig,
    ) -> Result<LipSyncBackend<B>, String> {
        LipSyncBackend::new(
            backend,
            config,
            self.levels.as_ref().ok_or("VTS driver is closed")?.clone(),
        )
    }

    pub fn status(&self) -> watch::Receiver<AvatarStatus> {
        self.status.clone()
    }

    pub fn mouth_parameter(&self) -> MouthControl {
        self.mouth.clone()
    }

    /// Caller drops the playback backend first, so no producer keeps the worker alive.
    pub async fn shutdown(mut self) {
        if let Some(levels) = self.levels.take() {
            levels.send_replace(0.0);
        }
        if let Some(mut task) = self.task.take() {
            if tokio::time::timeout(self.shutdown_timeout, &mut task)
                .await
                .is_err()
            {
                task.abort();
                let _ = task.await;
            }
        }
    }
}

impl Drop for AvatarDriver {
    fn drop(&mut self) {
        // Cancellation closes the final producer after playback drops; the actor
        // observes channel closure and performs its own bounded zero reset.
        if let Some(levels) = self.levels.take() {
            levels.send_replace(0.0);
        }
    }
}
