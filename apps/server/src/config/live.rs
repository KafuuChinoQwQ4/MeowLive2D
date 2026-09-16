//! 直播接入开关和本机凭据环境变量名；不保存密钥明文。
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LiveConfig {
    pub enabled: bool,
    pub app_id: u64,
    pub access_key_id_env: String,
    pub access_key_secret_env: String,
    pub identity_code_env: String,
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
            reconnect_initial_ms: 1000,
            reconnect_max_ms: 30000,
        }
    }
}
impl LiveConfig {
    pub fn validate(&self) -> Result<(), String> {
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
