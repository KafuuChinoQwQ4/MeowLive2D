use super::protocol::DecodeLimits;
use std::{net::IpAddr, time::Duration};

#[derive(Clone)]
pub struct BilibiliConfig {
    pub app_id: u64,
    pub access_key_id: String,
    pub access_key_secret: String,
    pub identity_code: String,
    pub api_base_url: String,
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
    pub receive_timeout: Duration,
    pub app_heartbeat_interval: Duration,
    pub websocket_heartbeat_interval: Duration,
    pub max_response_bytes: usize,
    pub decode_limits: DecodeLimits,
}

impl Default for BilibiliConfig {
    fn default() -> Self {
        Self {
            app_id: 0,
            access_key_id: String::new(),
            access_key_secret: String::new(),
            identity_code: String::new(),
            api_base_url: "https://live-open.biliapi.com".into(),
            request_timeout: Duration::from_secs(10),
            connect_timeout: Duration::from_secs(10),
            receive_timeout: Duration::from_secs(45),
            app_heartbeat_interval: Duration::from_secs(20),
            websocket_heartbeat_interval: Duration::from_secs(20),
            max_response_bytes: 256 * 1024,
            decode_limits: DecodeLimits::default(),
        }
    }
}

impl BilibiliConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.app_id == 0
            || self.access_key_id.trim().is_empty()
            || self.access_key_secret.is_empty()
            || self.identity_code.trim().is_empty()
            || self.access_key_id.len() > 256
            || self.access_key_secret.len() > 512
            || self.identity_code.len() > 512
        {
            return Err("哔哩哔哩直播凭据未完整配置".into());
        }
        let url = reqwest::Url::parse(&self.api_base_url)
            .map_err(|_| "哔哩哔哩 API 地址无效".to_string())?;
        match url.scheme() {
            "https" => {}
            "http" if loopback_host(&url) => {}
            _ => return Err("哔哩哔哩 API 必须使用 HTTPS；HTTP 仅允许本机测试".into()),
        }
        if url.query().is_some() || url.fragment().is_some() || url.cannot_be_a_base() {
            return Err("哔哩哔哩 API 地址不能包含查询或片段".into());
        }
        if !bounded_duration(self.request_timeout, 1, 60)
            || !bounded_duration(self.connect_timeout, 1, 60)
            || !bounded_duration(self.receive_timeout, 1, 120)
            || !bounded_duration(self.app_heartbeat_interval, 1, 60)
            || !bounded_duration(self.websocket_heartbeat_interval, 1, 60)
            || !(1024..=1024 * 1024).contains(&self.max_response_bytes)
        {
            return Err("哔哩哔哩连接超时、心跳或响应上限无效".into());
        }
        self.decode_limits.validate()
    }
}

pub(crate) fn loopback_host(url: &reqwest::Url) -> bool {
    url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    })
}

fn bounded_duration(value: Duration, min: u64, max: u64) -> bool {
    value >= Duration::from_secs(min) && value <= Duration::from_secs(max)
}
