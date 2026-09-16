//! OBS 配置只保存环境变量名，连接仅允许本机字面量 IP。
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ObsConfig {
    pub enabled: bool,
    pub websocket_url: String,
    pub password_env: String,
    pub timeout_ms: u64,
}

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            websocket_url: "ws://127.0.0.1:4455".into(),
            password_env: "MEOWLIVE_OBS_PASSWORD".into(),
            timeout_ms: 5000,
        }
    }
}

impl ObsConfig {
    pub fn validate(&self) -> Result<(), String> {
        // WHATWG URL parsing normalizes short numeric hosts, empty userinfo and
        // dot segments. Validate the original spelling before that can hide them.
        let endpoint = self
            .websocket_url
            .strip_prefix("ws://")
            .ok_or("OBS WebSocket 地址格式无效")?;
        let (authority, path) = endpoint.split_once('/').unwrap_or((endpoint, ""));
        if !path.is_empty() || authority.contains(['@', '\\', '?', '#']) {
            return Err("OBS 地址不能包含凭据、路径或查询参数".into());
        }
        let host = if let Some(ipv6) = authority.strip_prefix('[') {
            ipv6.split_once(']').map(|(host, _)| host)
        } else {
            authority.split(':').next()
        };
        if !host
            .and_then(|host| host.parse::<std::net::IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback())
        {
            return Err("OBS 地址须使用本机字面量 IP".into());
        }
        let url = url::Url::parse(&self.websocket_url).map_err(|_| "OBS WebSocket 地址格式无效")?;
        let loopback = match url.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if url.scheme() != "ws"
            || !loopback
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || self.websocket_url.chars().any(char::is_whitespace)
        {
            return Err("OBS 地址须为本机 IP 的 ws:// 地址，不能包含凭据、路径或查询参数".into());
        }
        if !(100..=10_000).contains(&self.timeout_ms) {
            return Err("OBS 操作超时须为 100 至 10000 毫秒".into());
        }
        if self.password_env.is_empty()
            || self.password_env.len() > 128
            || !self
                .password_env
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || self.password_env.as_bytes()[0].is_ascii_digit()
        {
            return Err("OBS 密码环境变量名无效".into());
        }
        Ok(())
    }
}
