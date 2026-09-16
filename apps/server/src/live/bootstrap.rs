//! 从环境变量组装官方直播源；缺少直播凭据不阻断人工播报和模拟事件。
use crate::config::LiveConfig;
use meowlive_adapters::live::bilibili::{BilibiliConfig, BilibiliLiveSource};
use meowlive_application::ports::live_source::LiveSource;
use std::sync::Arc;

pub fn build_live_source(config: &LiveConfig) -> Result<Option<Arc<dyn LiveSource>>, String> {
    config.validate()?;
    if !config.enabled {
        return Ok(None);
    }
    let read = |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    };
    let (Some(access_key_id), Some(access_key_secret), Some(identity_code)) = (
        read(&config.access_key_id_env),
        read(&config.access_key_secret_env),
        read(&config.identity_code_env),
    ) else {
        return Ok(None);
    };
    let source = BilibiliLiveSource::new(BilibiliConfig {
        app_id: config.app_id,
        access_key_id,
        access_key_secret,
        identity_code,
        ..BilibiliConfig::default()
    })
    .map_err(|_| "直播连接配置无效，请检查环境变量内容".to_owned())?;
    Ok(Some(Arc::new(source)))
}
