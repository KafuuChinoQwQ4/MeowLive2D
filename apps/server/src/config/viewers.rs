//! 默认开启的观众与记忆持久化、稳定角色范围和私有数据库变量引用。
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ViewersConfig {
    pub enabled: bool,
    pub receipt_directory: std::path::PathBuf,
    pub scope_id: String,
    pub database_url_env: String,
    pub calendar_offset_minutes: i32,
}

impl Default for ViewersConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            receipt_directory: "../data/viewer-receipts".into(),
            calendar_offset_minutes: 480,
            scope_id: "default-avatar".into(),
            database_url_env: "MEOWLIVE_DATABASE_URL".into(),
        }
    }
}

impl ViewersConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.receipt_directory.as_os_str().is_empty()
            || self.receipt_directory.to_string_lossy().len() > 4096
        {
            return Err("完成回执目录无效".into());
        }
        if !(-840..=840).contains(&self.calendar_offset_minutes) {
            return Err("观众日历 UTC 偏移须在 -840 到 840 分钟之间".into());
        }
        if self.scope_id.is_empty()
            || self.scope_id.len() > 64
            || !self
                .scope_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err("scope_id 须为 1–64 位字母、数字、点、横线或下划线".into());
        }
        if self.database_url_env.is_empty()
            || self.database_url_env.len() > 128
            || !self
                .database_url_env
                .bytes()
                .enumerate()
                .all(|(i, b)| b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit()))
        {
            return Err("database_url_env 须为有效环境变量名".into());
        }
        Ok(())
    }
}
