//! 可重建图副本配置，私有凭据仅由服务器环境读取。
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GraphConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub user: String,
    pub password_env: String,
}
impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://127.0.0.1:17474/db/neo4j/query/v2".into(),
            user: "neo4j".into(),
            password_env: "MEOWLIVE_NEO4J_PASSWORD".into(),
        }
    }
}
impl GraphConfig {
    pub fn validate(&self, viewers: bool) -> Result<(), String> {
        if self.enabled
            && (!viewers || self.user.trim().is_empty() || self.password_env.trim().is_empty())
        {
            return Err("图同步需要观众存储和私有凭据".into());
        }
        let uri = self
            .endpoint
            .parse::<axum::http::Uri>()
            .map_err(|_| "图服务地址无效")?;
        if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.host().is_none() {
            return Err("图服务必须使用 HTTP(S) 地址".into());
        }
        Ok(())
    }
}
