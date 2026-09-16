//! 将有界资源请求绑定到当前桌面连接；锁外等待，超时与取消清除关联。
use crate::{state::AppState, transport::error::ApiError};
use axum::http::StatusCode;
use meowlive_protocol::{
    control::ServerCommand,
    resources::{DesktopResourceOperation, DesktopResourceResult},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::oneshot;

pub(crate) type PendingRequests = HashMap<String, (String, oneshot::Sender<DesktopResourceResult>)>;
struct Pending {
    id: String,
    requests: Arc<Mutex<PendingRequests>>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.requests
            .lock()
            .expect("resource requests lock")
            .remove(&self.id);
    }
}

impl AppState {
    pub(crate) async fn desktop_resource(
        &self,
        operation: DesktopResourceOperation,
    ) -> Result<DesktopResourceResult, ApiError> {
        let _permit = self.resource_requests.try_acquire().map_err(|_| {
            ApiError::new(
                StatusCode::CONFLICT,
                "resource_busy",
                "桌面资源操作正在进行，请稍后重试",
            )
        })?;
        let import = matches!(operation, DesktopResourceOperation::ImportModel);
        let (bridge, sender, cancel, generation) = {
            let inner = self.inner.lock().await;
            let b = inner
                .bridge
                .as_ref()
                .filter(|_| inner.queue.is_connected())
                .ok_or_else(|| {
                    ApiError::new(
                        StatusCode::CONFLICT,
                        "bridge_disconnected",
                        "桌面执行端尚未连接",
                    )
                })?;
            (
                b.id.clone(),
                b.control.clone(),
                b.cancel.clone(),
                inner.generation_cancel.clone(),
            )
        };
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.resource_pending
            .lock()
            .expect("resource requests lock")
            .insert(id.clone(), (bridge, tx));
        let _pending = Pending {
            id: id.clone(),
            requests: self.resource_pending.clone(),
        };
        sender
            .try_send(ServerCommand::Resource {
                request_id: id,
                operation,
            })
            .map_err(|_| {
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "desktop_busy",
                    "桌面控制通道暂不可用",
                )
            })?;
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(ApiError::new(StatusCode::CONFLICT, "bridge_disconnected", "桌面已断开，操作结果未知，请重新查询资源")),
            _ = generation.cancelled() => Err(ApiError::new(StatusCode::CONFLICT, "resource_cancelled", "资源操作已取消，若模型已安装请刷新列表确认")),
            result = tokio::time::timeout(Duration::from_secs(if import {90} else {15}), rx) => result.ok().and_then(Result::ok).ok_or_else(|| ApiError::new(StatusCode::GATEWAY_TIMEOUT, "resource_timeout", "桌面资源操作超时，结果未知，请刷新资源后重试")),
        }
    }

    pub(crate) fn resource_reply(&self, bridge: &str, id: String, result: DesktopResourceResult) {
        let mut pending = self
            .resource_pending
            .lock()
            .expect("resource requests lock");
        if pending.get(&id).is_some_and(|(owner, _)| owner == bridge) {
            if let Some((_, sender)) = pending.remove(&id) {
                let _ = sender.send(result);
            }
        }
    }
}
