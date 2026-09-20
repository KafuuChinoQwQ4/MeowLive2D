//! 直播接入配置；面板凭据由独立本机文件注入，TOML 保留环境变量兼容入口。
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LiveCredentials {
    pub access_key_id: Option<String>,
    pub access_key_secret: Option<String>,
    pub identity_code: Option<String>,
}

impl std::fmt::Debug for LiveCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveCredentials")
            .field("access_key_id_configured", &self.access_key_id.is_some())
            .field(
                "access_key_secret_configured",
                &self.access_key_secret.is_some(),
            )
            .field("identity_code_configured", &self.identity_code.is_some())
            .finish()
    }
}

impl LiveCredentials {
    pub fn validate(&self) -> Result<(), String> {
        for (value, limit, label) in [
            (&self.access_key_id, 256, "开发者 AccessKey ID"),
            (&self.access_key_secret, 512, "开发者 AccessKey Secret"),
            (&self.identity_code, 512, "主播身份码"),
        ] {
            if value.as_ref().is_some_and(|value| {
                value.trim().is_empty()
                    || value.len() > limit
                    || value.chars().any(char::is_control)
            }) {
                return Err(format!("{label}无效或超过长度上限"));
            }
        }
        Ok(())
    }
    pub fn complete(&self) -> bool {
        self.access_key_id.is_some()
            && self.access_key_secret.is_some()
            && self.identity_code.is_some()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LiveConfig {
    pub enabled: bool,
    pub app_id: u64,
    pub access_key_id_env: String,
    pub access_key_secret_env: String,
    pub identity_code_env: String,
    #[serde(skip)]
    pub credentials: Option<LiveCredentials>,
    pub reconnect_initial_ms: u64,
    pub reconnect_max_ms: u64,
}
impl Default for LiveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            app_id: 0,
            access_key_id_env: "MEOWLIVE_BILIBILI_ACCESS_KEY_ID".into(),
            access_key_secret_env: "MEOWLIVE_BILIBILI_ACCESS_KEY_SECRET".into(),
            identity_code_env: "MEOWLIVE_BILIBILI_IDENTITY_CODE".into(),
            credentials: None,
            reconnect_initial_ms: 1000,
            reconnect_max_ms: 30000,
        }
    }
}
impl LiveConfig {
    pub fn resolved_credentials(&self) -> LiveCredentials {
        self.credentials.clone().unwrap_or_else(|| {
            let read = |name: &str| {
                std::env::var(name)
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            };
            LiveCredentials {
                access_key_id: read(&self.access_key_id_env),
                access_key_secret: read(&self.access_key_secret_env),
                identity_code: read(&self.identity_code_env),
            }
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if let Some(credentials) = &self.credentials {
            credentials.validate()?;
        }
        if self.enabled && (self.app_id == 0 || self.app_id > i64::MAX as u64) {
            return Err("启用直播接入时，live.app_id 须为有效正整数".into());
        }
        for name in [
            &self.access_key_id_env,
            &self.access_key_secret_env,
            &self.identity_code_env,
        ] {
            if name.is_empty()
                || name.len() > 128
                || !name.bytes().enumerate().all(|(i, c)| {
                    c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                })
            {
                return Err("直播凭据须配置有效的环境变量名".into());
            }
        }
        if !(10..=60_000).contains(&self.reconnect_initial_ms)
            || !(self.reconnect_initial_ms..=300_000).contains(&self.reconnect_max_ms)
        {
            return Err(
                "直播重连初始间隔须为 10–60000 ms，上限不小于初始间隔且不超过 300000 ms".into(),
            );
        }
        Ok(())
    }
}
