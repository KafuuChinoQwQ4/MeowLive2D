//! 从本机面板设置或兼容环境变量组装官方直播源。
use crate::config::LiveConfig;
use meowlive_adapters::live::bilibili::{BilibiliConfig, BilibiliLiveSource};
use meowlive_application::ports::live_source::LiveSource;
use std::sync::Arc;

pub fn build_live_source(config: &LiveConfig) -> Result<Option<Arc<dyn LiveSource>>, String> {
    config.validate()?;
    if !config.enabled {
        return Ok(None);
    }
    let credentials = config.resolved_credentials();
    credentials.validate()?;
    let (Some(access_key_id), Some(access_key_secret), Some(identity_code)) = (
        credentials.access_key_id,
        credentials.access_key_secret,
        credentials.identity_code,
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
