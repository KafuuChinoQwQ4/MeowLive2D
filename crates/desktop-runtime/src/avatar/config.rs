use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VtsConfig {
    pub enabled: bool,
    pub websocket_url: String,
    pub token_path: PathBuf,
    pub mouth_parameter: String,
    pub request_timeout_ms: u64,
    pub auth_timeout_ms: u64,
    pub reconnect_delay_ms: u64,
}
impl Default for VtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            websocket_url: "ws://127.0.0.1:8001".into(),
            token_path: "local/vts-token.json".into(),
            mouth_parameter: "MeowMouthOpen".into(),
            request_timeout_ms: 1000,
            auth_timeout_ms: 60000,
            reconnect_delay_ms: 2000,
        }
    }
}
impl VtsConfig {
    pub fn validate(&self) -> Result<(), String> {
        let url = url::Url::parse(&self.websocket_url).map_err(|_| "VTS WebSocket 地址格式无效")?;
        if url.scheme() != "ws"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || self.websocket_url.chars().any(char::is_whitespace)
        {
            return Err("VTS 地址须为不含凭据、查询参数或片段的 ws:// 地址".into());
        }
        if !(4..=32).contains(&self.mouth_parameter.len())
            || !self
                .mouth_parameter
                .bytes()
                .all(|c| c.is_ascii_alphanumeric())
        {
            return Err("VTS 口型参数须为 4 至 32 位 ASCII 字母或数字".into());
        }
        if self.token_path.as_os_str().is_empty() {
            return Err("VTS 本地授权文件路径不能为空".into());
        }
        if !(10..=30000).contains(&self.request_timeout_ms) {
            return Err("VTS 请求超时须为 10 至 30000 毫秒".into());
        }
        if !(100..=300000).contains(&self.auth_timeout_ms) {
            return Err("VTS 授权超时须为 100 至 300000 毫秒".into());
        }
        if !(10..=60000).contains(&self.reconnect_delay_ms) {
            return Err("VTS 重连间隔须为 10 至 60000 毫秒".into());
        }
        Ok(())
    }
}
