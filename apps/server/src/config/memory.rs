//! 后台记忆模型能力的显式配置；不继承实时模型密钥。
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MemoryConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub model: String,
    pub api_key_env: String,
    pub embedding_endpoint: String,
    pub embedding_model: String,
    pub embedding_api_key_env: String,
    pub embedding_dimensions: usize,
    pub timeout_ms: u64,
}
impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: String::new(),
            model: String::new(),
            api_key_env: "MEOWLIVE_MEMORY_API_KEY".into(),
            embedding_endpoint: String::new(),
            embedding_model: String::new(),
            embedding_api_key_env: "MEOWLIVE_EMBEDDING_API_KEY".into(),
            embedding_dimensions: 384,
            timeout_ms: 15000,
        }
    }
}
impl MemoryConfig {
    pub fn validate(&self, viewers: bool) -> Result<(), String> {
        if self.enabled && (!viewers || self.endpoint.is_empty() || self.model.is_empty()) {
            return Err("记忆提取要求启用观众存储并设置独立接口与模型".into());
        }
        if self.embedding_dimensions == 0
            || self.embedding_dimensions > 4096
            || !(1..=60000).contains(&self.timeout_ms)
        {
            return Err("记忆模型维度或超时超出范围".into());
        }
        if self.embedding_endpoint.is_empty() != self.embedding_model.is_empty() {
            return Err("嵌入接口与模型必须一起设置".into());
        }
        Ok(())
    }
}
