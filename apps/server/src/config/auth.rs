//! 管理员与执行设备认证的私有凭据来源和有界会话配置。
use serde::Deserialize;
use std::path::PathBuf;

pub const MAX_SESSION_LIFETIME_SECONDS: u32 = 8 * 60 * 60;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthConfig {
    pub enabled: bool,
    pub admin_token_env: String,
    pub device_token_env: String,
    pub admin_token_file: Option<PathBuf>,
    pub device_token_file: Option<PathBuf>,
    pub session_lifetime_seconds: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            admin_token_env: "MEOWLIVE_ADMIN_TOKEN".into(),
            device_token_env: "MEOWLIVE_DEVICE_TOKEN".into(),
            admin_token_file: None,
            device_token_file: None,
            session_lifetime_seconds: MAX_SESSION_LIFETIME_SECONDS,
        }
    }
}

impl AuthConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=MAX_SESSION_LIFETIME_SECONDS).contains(&self.session_lifetime_seconds) {
            return Err("session_lifetime_seconds 必须在 1..28800".into());
        }
        for name in [&self.admin_token_env, &self.device_token_env] {
            if !name.is_empty() && !valid_env_name(name) {
                return Err("认证凭据环境变量名无效".into());
            }
        }
        if self.enabled {
            if self.admin_token_env.is_empty() && self.admin_token_file.is_none() {
                return Err("启用认证时必须配置管理员凭据来源".into());
            }
            if self.device_token_env.is_empty() && self.device_token_file.is_none() {
                return Err("启用认证时必须配置设备凭据来源".into());
            }
        }
        for path in [&self.admin_token_file, &self.device_token_file]
            .into_iter()
            .flatten()
        {
            if path.as_os_str().is_empty() {
                return Err("认证凭据文件路径不能为空".into());
            }
        }
        Ok(())
    }
}

fn valid_env_name(name: &str) -> bool {
    name.bytes().enumerate().all(|(index, byte)| {
        byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit())
    })
}
