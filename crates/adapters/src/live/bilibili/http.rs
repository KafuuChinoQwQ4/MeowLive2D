//! 签名 HTTP 生命周期调用；响应读取和错误内容均有界且脱敏。

use super::{BilibiliConfig, signing};
use futures_util::StreamExt;
use meowlive_application::ports::live_source::LiveSourceError;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub(crate) struct ApiClient {
    client: reqwest::Client,
    config: BilibiliConfig,
}

#[derive(Clone)]
pub(crate) struct StartedProject {
    pub game_id: String,
    pub auth_body: String,
    pub websocket_urls: Vec<String>,
    pub room_id: String,
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    code: i64,
    data: Option<T>,
}

#[derive(Deserialize)]
struct StartData {
    game_info: GameInfo,
    websocket_info: WebsocketInfo,
    anchor_info: AnchorInfo,
}

#[derive(Deserialize)]
struct GameInfo {
    game_id: String,
}

#[derive(Deserialize)]
struct WebsocketInfo {
    auth_body: String,
    wss_link: Vec<String>,
}

#[derive(Deserialize)]
struct AnchorInfo {
    room_id: u64,
}

#[derive(Serialize)]
struct StartBody<'a> {
    code: &'a str,
    app_id: u64,
}

#[derive(Serialize)]
struct ProjectBody<'a> {
    app_id: u64,
    game_id: &'a str,
}

#[derive(Serialize)]
struct HeartbeatBody<'a> {
    game_id: &'a str,
}

impl ApiClient {
    pub fn new(config: BilibiliConfig) -> Result<Self, LiveSourceError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.request_timeout)
            .build()
            .map_err(|_| error("无法创建哔哩哔哩 HTTP 客户端", false))?;
        Ok(Self { client, config })
    }

    pub async fn start(&self) -> Result<StartedProject, LiveSourceError> {
        let body = serde_json::to_vec(&StartBody {
            code: &self.config.identity_code,
            app_id: self.config.app_id,
        })
        .map_err(|_| error("无法编码哔哩哔哩启动请求", false))?;
        let response: ApiResponse<serde_json::Value> =
            self.post("/v2/app/start", body, true).await?;
        if response.code != 0 {
            if let Some(delay) = super::protocol::api_retry_delay(response.code) {
                tokio::time::sleep(delay).await;
            }
            return Err(business_error(response.code, "哔哩哔哩拒绝启动直播项目"));
        }
        let raw = response
            .data
            .ok_or_else(|| error("哔哩哔哩启动响应缺少数据", false))?;
        let known_game = raw
            .get("game_info")
            .and_then(|game| game.get("game_id"))
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 128)
            .map(str::to_owned);
        let data: StartData = match serde_json::from_value(raw) {
            Ok(data) => data,
            Err(_) => {
                if let Some(game_id) = known_game {
                    self.end(&game_id)
                        .await
                        .map_err(|_| error("启动响应不完整且结束直播会话失败", false))?;
                }
                return Err(error("哔哩哔哩启动请求结果未知", false));
            }
        };
        if data.game_info.game_id.is_empty()
            || data.game_info.game_id.len() > 128
            || data.websocket_info.auth_body.is_empty()
            || data.websocket_info.auth_body.len() > 64 * 1024
            || data.websocket_info.wss_link.is_empty()
            || data.websocket_info.wss_link.len() > 8
            || data.anchor_info.room_id == 0
        {
            if !data.game_info.game_id.is_empty() && data.game_info.game_id.len() <= 128 {
                self.end(&data.game_info.game_id)
                    .await
                    .map_err(|_| error("启动响应不完整且结束直播会话失败", false))?;
            }
            return Err(error("哔哩哔哩启动响应不完整", false));
        }
        Ok(StartedProject {
            game_id: data.game_info.game_id,
            auth_body: data.websocket_info.auth_body,
            websocket_urls: data.websocket_info.wss_link,
            room_id: data.anchor_info.room_id.to_string(),
        })
    }

    pub async fn heartbeat(&self, game_id: &str) -> Result<(), LiveSourceError> {
        let body = serde_json::to_vec(&HeartbeatBody { game_id })
            .map_err(|_| error("无法编码哔哩哔哩心跳请求", false))?;
        self.expect_empty("/v2/app/heartbeat", body, "哔哩哔哩项目心跳被拒绝", false)
            .await
    }

    pub async fn end(&self, game_id: &str) -> Result<(), LiveSourceError> {
        let body = serde_json::to_vec(&ProjectBody {
            app_id: self.config.app_id,
            game_id,
        })
        .map_err(|_| error("无法编码哔哩哔哩结束请求", false))?;
        self.expect_empty("/v2/app/end", body, "哔哩哔哩拒绝结束直播项目", true)
            .await
    }

    async fn expect_empty(
        &self,
        path: &str,
        body: Vec<u8>,
        rejected: &'static str,
        already_ended_is_ok: bool,
    ) -> Result<(), LiveSourceError> {
        let response: ApiResponse<serde_json::Value> = self.post(path, body, false).await?;
        if response.code == 0 || (already_ended_is_ok && response.code == 7003) {
            Ok(())
        } else {
            Err(business_error(response.code, rejected))
        }
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Vec<u8>,
        start_is_non_idempotent: bool,
    ) -> Result<T, LiveSourceError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| error("系统时间无法用于哔哩哔哩签名", false))?
            .as_secs();
        let nonce = uuid::Uuid::new_v4().to_string();
        let signed = signing::sign(
            &self.config.access_key_id,
            &self.config.access_key_secret,
            &body,
            timestamp,
            &nonce,
        )
        .map_err(|message| error(message, false))?;
        let headers = signed_headers(&signed)?;
        let url = format!("{}{}", self.config.api_base_url.trim_end_matches('/'), path);
        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|cause| {
                if start_is_non_idempotent && cause.is_timeout() {
                    error("哔哩哔哩启动请求结果未知", false)
                } else {
                    error("哔哩哔哩 HTTP 请求失败", true)
                }
            })?;
        if !response.status().is_success() {
            let retryable = response.status().is_server_error()
                || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS;
            return Err(error("哔哩哔哩 HTTP 状态异常", retryable));
        }
        let bytes = bounded_body(response, self.config.max_response_bytes)
            .await
            .map_err(|cause| {
                if start_is_non_idempotent {
                    error("哔哩哔哩启动请求结果未知", false)
                } else {
                    cause
                }
            })?;
        serde_json::from_slice(&bytes).map_err(|_| {
            if start_is_non_idempotent {
                error("哔哩哔哩启动请求结果未知", false)
            } else {
                error("哔哩哔哩 HTTP 响应格式无效", false)
            }
        })
    }
}

fn business_error(code: i64, message: &'static str) -> LiveSourceError {
    error(message, matches!(code, 4003 | 4004 | 5000..=5003 | 7001))
}

fn signed_headers(signed: &signing::SignedHeaders) -> Result<HeaderMap, LiveSourceError> {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("x-bili-content-md5", signed.content_md5.as_str()),
        ("x-bili-timestamp", signed.timestamp.as_str()),
        ("x-bili-signature-method", "HMAC-SHA256"),
        ("x-bili-signature-nonce", signed.nonce.as_str()),
        ("x-bili-accesskeyid", signed.access_key_id.as_str()),
        ("x-bili-signature-version", "1.0"),
    ] {
        headers.insert(
            reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_str(value).map_err(|_| error("哔哩哔哩签名请求头无效", false))?,
        );
    }
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&signed.authorization)
            .map_err(|_| error("哔哩哔哩签名结果无效", false))?,
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
}

async fn bounded_body(
    response: reqwest::Response,
    maximum: usize,
) -> Result<Vec<u8>, LiveSourceError> {
    let mut stream = response.bytes_stream();
    let mut output = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| error("读取哔哩哔哩 HTTP 响应失败", true))?;
        if output.len().saturating_add(chunk.len()) > maximum {
            return Err(error("哔哩哔哩 HTTP 响应超过上限", false));
        }
        output.extend_from_slice(&chunk);
    }
    Ok(output)
}

pub(crate) fn error(message: impl Into<String>, retryable: bool) -> LiveSourceError {
    LiveSourceError::new(message, retryable)
}
