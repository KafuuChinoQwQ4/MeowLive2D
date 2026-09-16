//! 独立连接状态机：只消费 watch 最新值，重连清除旧样本，停止优先归零。
use super::{
    AvatarStatus, VtsConfig,
    client::{Client, Error},
    token,
};
use serde_json::json;
use std::{future::Future, time::Duration};
use tokio::{sync::watch, time::Instant};

const FRAME: Duration = Duration::from_nanos(33_333_334);
const FRESHNESS: Duration = Duration::from_millis(250);
const RESET_TIMEOUT: Duration = Duration::from_millis(250);

struct Levels {
    receiver: watch::Receiver<f64>,
    value: f64,
    updated: Option<Instant>,
}
impl Levels {
    fn clear(&mut self) {
        self.receiver.borrow_and_update();
        self.value = 0.0;
        self.updated = None;
    }
    fn current(&self) -> f64 {
        if self.updated.is_some_and(|time| time.elapsed() <= FRESHNESS) {
            self.value
        } else {
            0.0
        }
    }
    async fn during<T>(
        &mut self,
        operation: impl Future<Output = Result<T, Error>>,
        interrupt_reset: bool,
    ) -> Result<T, Error> {
        tokio::pin!(operation);
        loop {
            let stale_at = self.updated.unwrap_or_else(Instant::now) + FRESHNESS;
            tokio::select! {
                biased;
                changed = self.receiver.changed() => {
                    changed.map_err(|_| Error::Shutdown)?;
                    let next = *self.receiver.borrow_and_update();
                    let next = if next.is_finite() { next.clamp(0.0, 1.0) } else { 0.0 };
                    let reset = self.value != 0.0 && next == 0.0;
                    self.value = next;
                    self.updated = Some(Instant::now());
                    if interrupt_reset && reset { return Err(Error::Reset); }
                }
                _ = tokio::time::sleep_until(stale_at), if interrupt_reset && self.value != 0.0 && self.updated.is_some() => {
                    self.value = 0.0;
                    self.updated = None;
                    return Err(Error::Reset);
                }
                result = &mut operation => return result,
            }
        }
    }
}

/// Runs independently of the audio callback. Closing `levels` performs a bounded
/// zero reset; authorization denial requires an explicit application restart.
pub async fn run(
    config: VtsConfig,
    levels: watch::Receiver<f64>,
    status: watch::Sender<AvatarStatus>,
) {
    if !config.enabled {
        status.send_replace(AvatarStatus::Disabled);
        return;
    }
    if let Err(error) = config.validate() {
        status.send_replace(AvatarStatus::Failed(error));
        return;
    }
    let mut levels = Levels {
        receiver: levels,
        value: 0.0,
        updated: None,
    };
    let load = async {
        token::load(config.token_path.clone())
            .await
            .map_err(Error::Storage)
    };
    let mut cached = match levels.during(load, false).await {
        Ok(token) => token,
        Err(Error::Shutdown) => {
            status.send_replace(AvatarStatus::Disconnected);
            return;
        }
        Err(error) => {
            status.send_replace(AvatarStatus::Failed(error.message()));
            return;
        }
    };
    let mut needs_save = false;
    let deadline = Duration::from_millis(config.request_timeout_ms);
    loop {
        if levels.receiver.has_changed().is_err() {
            return;
        }
        status.send_replace(AvatarStatus::Connecting);
        let connection = levels
            .during(Client::connect(&config.websocket_url, deadline), false)
            .await;
        let result = match connection {
            Ok(mut client) => {
                match setup(
                    &mut client,
                    &config,
                    &mut cached,
                    &mut needs_save,
                    &mut levels,
                    &status,
                )
                .await
                {
                    Ok(()) => {
                        levels.clear();
                        let result = stream(&mut client, &config, &mut levels, &status).await;
                        // A canceled request can still receive its old reply. Correlation
                        // ignores it while the final zero is sent with a new request ID.
                        let _ = client
                            .inject(&config.mouth_parameter, 0.0, deadline.min(RESET_TIMEOUT))
                            .await;
                        result
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        };
        match result {
            Err(Error::Shutdown) => {
                status.send_replace(AvatarStatus::Disconnected);
                return;
            }
            Err(Error::Authorization) => {
                status.send_replace(AvatarStatus::AuthorizationRequired);
                return;
            }
            Err(error @ (Error::Protocol(_) | Error::Api(_) | Error::Storage(_))) => {
                status.send_replace(AvatarStatus::Failed(error.message()));
                return;
            }
            _ => status.send_replace(AvatarStatus::Disconnected),
        };
        let delay = async {
            tokio::time::sleep(Duration::from_millis(config.reconnect_delay_ms)).await;
            Ok(())
        };
        if levels.during(delay, false).await.is_err() {
            return;
        }
    }
}

async fn setup(
    client: &mut Client,
    config: &VtsConfig,
    cached: &mut Option<String>,
    needs_save: &mut bool,
    levels: &mut Levels,
    status: &watch::Sender<AvatarStatus>,
) -> Result<(), Error> {
    let deadline = Duration::from_millis(config.request_timeout_ms);
    let credential = match cached.as_ref() {
        Some(token) => token.clone(),
        None => {
            status.send_replace(AvatarStatus::AwaitingAuthorization);
            let response = levels
                .during(
                    client.request(
                        "AuthenticationTokenRequest",
                        "AuthenticationTokenResponse",
                        json!({"pluginName":"MeowLive2D","pluginDeveloper":"MeowLive2D"}),
                        Duration::from_millis(config.auth_timeout_ms),
                    ),
                    false,
                )
                .await;
            // An interrupted token dialog must never be automatically requested again.
            let response = response.map_err(|error| match error {
                Error::Timeout | Error::Transport => Error::Authorization,
                other => other,
            })?;
            let token = response["authenticationToken"]
                .as_str()
                .ok_or(Error::Protocol("VTS 授权响应缺少令牌"))?;
            token::validate(token).map_err(|_| Error::Protocol("VTS 授权响应令牌格式无效"))?;
            // Retain a granted token across a transport failure in the following
            // authentication request, without opening another authorization dialog.
            *cached = Some(token.to_owned());
            *needs_save = true;
            token.to_owned()
        }
    };
    let response = levels.during(client.request("AuthenticationRequest", "AuthenticationResponse",
        json!({"pluginName":"MeowLive2D","pluginDeveloper":"MeowLive2D","authenticationToken":credential}), deadline), false).await?;
    match response["authenticated"].as_bool() {
        Some(true) => {}
        Some(false) => return Err(Error::Authorization),
        None => return Err(Error::Protocol("VTS 认证响应缺少结果")),
    }
    if *needs_save {
        let save = async {
            token::save(config.token_path.clone(), credential.clone())
                .await
                .map_err(Error::Storage)
        };
        levels.during(save, false).await?;
        *needs_save = false;
    }
    let response = levels.during(client.request("ParameterCreationRequest", "ParameterCreationResponse",
        json!({"parameterName":config.mouth_parameter,"explanation":"MeowLive2D playback mouth opening",
            "min":0.0,"max":1.0,"defaultValue":0.0}), deadline), false).await?;
    if response["parameterName"].as_str() != Some(&config.mouth_parameter) {
        return Err(Error::Protocol("VTS 创建参数响应名称不匹配"));
    }
    Ok(())
}

async fn stream(
    client: &mut Client,
    config: &VtsConfig,
    levels: &mut Levels,
    status: &watch::Sender<AvatarStatus>,
) -> Result<(), Error> {
    let deadline = Duration::from_millis(config.request_timeout_ms);
    levels
        .during(client.inject(&config.mouth_parameter, 0.0, deadline), false)
        .await?;
    status.send_replace(AvatarStatus::Connected);
    let mut next = Instant::now() + FRAME;
    loop {
        let wait = async {
            tokio::time::sleep_until(next).await;
            Ok(())
        };
        // Explicit zero bypasses the cadence to close promptly on Stop/Failure.
        match levels.during(wait, true).await {
            Ok(()) | Err(Error::Reset) => {}
            Err(error) => return Err(error),
        }
        let value = levels.current();
        next = Instant::now() + FRAME;
        match levels
            .during(
                client.inject(&config.mouth_parameter, value, deadline),
                true,
            )
            .await
        {
            Ok(()) => {}
            Err(Error::Reset) => next = Instant::now(),
            Err(error) => return Err(error),
        }
    }
}
