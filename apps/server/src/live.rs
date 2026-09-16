//! 直播连接的单会话所有权、公开状态和非阻塞控制；I/O 驱动独立于业务锁。
pub mod bootstrap;
mod worker;

use crate::{config::LiveConfig, state::AppState, transport::error::ApiError};
use axum::http::StatusCode;
use meowlive_protocol::live::{LiveConnectionPhase as Phase, LiveConnectionSnapshot};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub(crate) struct LiveState {
    pub snapshot: LiveConnectionSnapshot,
    pub cancel: Option<CancellationToken>,
    pub done: Option<watch::Receiver<bool>>,
}

impl LiveState {
    pub fn new(config: &LiveConfig, configured: bool) -> Self {
        Self {
            snapshot: LiveConnectionSnapshot {
                platform: "bilibili".into(),
                configured,
                phase: if config.enabled {
                    Phase::Disconnected
                } else {
                    Phase::Disabled
                },
                room_id: None,
                accepted_events: 0,
                duplicate_events: 0,
                rejected_events: 0,
                reconnect_attempts: 0,
                last_error: if config.enabled && !configured {
                    Some("直播凭据未配置，请设置主服务环境变量后重启。".into())
                } else {
                    None
                },
            },
            cancel: None,
            done: None,
        }
    }
}

impl AppState {
    pub async fn live_snapshot(&self) -> LiveConnectionSnapshot {
        self.inner.lock().await.live.snapshot.clone()
    }

    pub async fn connect_live(&self) -> Result<LiveConnectionSnapshot, ApiError> {
        let mut inner = self.inner.lock().await;
        self.require_gpu_idle()?;
        if self.stopping.is_cancelled() {
            return Err(conflict("server_stopping", "主服务正在退出"));
        }
        if !self.config.live.enabled || self.live_source.is_none() {
            return Err(conflict(
                "live_unconfigured",
                "请启用直播配置并设置凭据环境变量后重启主服务",
            ));
        }
        if inner.live.snapshot.phase == Phase::Disconnecting {
            return Err(conflict(
                "live_disconnecting",
                "正在结束上一次直播连接，请稍后重试",
            ));
        }
        if inner.live.cancel.is_some() {
            return Ok(inner.live.snapshot.clone());
        }
        let cancel = CancellationToken::new();
        let (done, receiver) = watch::channel(false);
        inner.live.cancel = Some(cancel.clone());
        inner.live.done = Some(receiver);
        inner.live.snapshot.phase = Phase::Connecting;
        inner.live.snapshot.room_id = None;
        inner.live.snapshot.last_error = None;
        let snapshot = inner.live.snapshot.clone();
        tokio::spawn(worker::run(self.clone(), cancel, done));
        Ok(snapshot)
    }

    pub async fn disconnect_live(&self) -> LiveConnectionSnapshot {
        let mut inner = self.inner.lock().await;
        if let Some(cancel) = &inner.live.cancel {
            cancel.cancel();
            inner.live.snapshot.phase = Phase::Disconnecting;
            inner.live.snapshot.room_id = None;
            inner.agent.set_paused(true, self.now_ms());
            inner.agent_cancel.cancel();
            inner.agent_cancel = CancellationToken::new();
            self.agent_wake.notify_one();
        }
        inner.live.snapshot.clone()
    }

    pub(crate) async fn wait_live_shutdown(&self) {
        let done = self.inner.lock().await.live.done.clone();
        if let Some(mut done) = done {
            let _ = done.wait_for(|finished| *finished).await;
        }
    }
}

fn conflict(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}
