//! 按角色切换口型输入；旧连接先归零退出，新连接再创建目标参数。
use super::{AvatarStatus, VtsConfig};
use std::time::Duration;
use tokio::{sync::watch, task::JoinHandle};

#[derive(Clone)]
pub struct MouthControl {
    pub(crate) requested: watch::Sender<String>,
    pub(crate) applied: watch::Receiver<Option<(String, Result<(), String>)>>,
}
impl MouthControl {
    pub async fn select(&self, parameter: String) -> Result<(), String> {
        let mut applied = self.applied.clone();
        if self.requested.receiver_count() == 0 {
            return Err("桌面口型驱动未运行".into());
        }
        self.requested.send_replace(parameter.clone());
        loop {
            if let Some((current, result)) = &*applied.borrow_and_update() {
                if current == &parameter {
                    return result.clone();
                }
            }
            applied.changed().await.map_err(|_| "口型驱动已停止")?;
        }
    }
}

async fn finish(sender: watch::Sender<f64>, mut task: JoinHandle<()>, timeout: Duration) {
    sender.send_replace(0.0);
    drop(sender);
    if tokio::time::timeout(timeout, &mut task).await.is_err() {
        task.abort();
        let _ = task.await;
    }
}

pub(crate) async fn run(
    mut config: VtsConfig,
    mut levels: watch::Receiver<f64>,
    mut parameters: watch::Receiver<String>,
    status: watch::Sender<AvatarStatus>,
    applied: watch::Sender<Option<(String, Result<(), String>)>>,
) {
    let timeout = Duration::from_millis(config.request_timeout_ms + 100);
    loop {
        config.mouth_parameter = parameters.borrow_and_update().clone();
        applied.send_replace(None);
        let (sender, receiver) = watch::channel(0.0);
        let (actor_status, mut observed) = watch::channel(AvatarStatus::Connecting);
        let task = tokio::spawn(super::run(config.clone(), receiver, actor_status));
        let mut observed_open = true;
        let restart = loop {
            tokio::select! {
                biased;
                changed=levels.changed()=>{
                    if changed.is_err(){break false;}
                    sender.send_replace(*levels.borrow_and_update());
                }
                changed=parameters.changed()=>{
                    if changed.is_err(){break false;}
                    if *parameters.borrow()!=config.mouth_parameter{break true;}
                    parameters.borrow_and_update();
                }
                changed=observed.changed(), if observed_open=>{
                    if changed.is_ok(){
                        let value=observed.borrow_and_update().clone();
                        let result=match &value {
                            AvatarStatus::Connected=>Some(Ok(())),
                            AvatarStatus::Failed(error)=>Some(Err(error.clone())),
                            AvatarStatus::AuthorizationRequired=>Some(Err("请在 VTS 中重新授权 MeowLive2D".into())),
                            AvatarStatus::Disabled=>Some(Err("VTS 口型驱动未启用".into())),
                            _=>None,
                        };
                        applied.send_replace(result.map(|result|(config.mouth_parameter.clone(),result)));
                        status.send_replace(value);
                    } else {observed_open=false;}
                }
            }
        };
        finish(sender, task, timeout).await;
        if !restart {
            return;
        }
    }
}
