//! 运行设置验证与搜索密钥的目标绑定；估算价格只接受明确配置。
use super::RuntimeState;
use meowlive_protocol::llm_runtime::{
    AgentRuntimeSettings, AgentRuntimeSettingsRequest, AgentRuntimeSettingsSnapshot,
};
use std::collections::HashSet;

pub(super) fn defaults() -> AgentRuntimeSettings {
    AgentRuntimeSettings {
        cache_enabled: true,
        streaming: true,
        tools_enabled: true,
        environment_enabled: true,
        web_search_enabled: false,
        search_provider: "brave".into(),
        search_endpoint: "https://api.search.brave.com/res/v1/web/search".into(),
        max_tool_rounds: 2,
        tool_timeout_seconds: 6,
        prices: vec![],
    }
}
pub(super) fn snapshot(state: &RuntimeState) -> AgentRuntimeSettingsSnapshot {
    AgentRuntimeSettingsSnapshot {
        settings: state.settings.clone(),
        search_key_configured: state.search_key.is_some(),
        storage_available: state.disk.is_some() && state.healthy,
    }
}
pub(super) fn resolve(
    state: &RuntimeState,
    request: AgentRuntimeSettingsRequest,
) -> Result<(AgentRuntimeSettings, Option<String>), String> {
    validate(&request.settings)?;
    if request.clear_search_api_key && request.search_api_key.is_some() {
        return Err("不能同时提交和清除搜索密钥".into());
    }
    let same_target = request.settings.search_provider == state.settings.search_provider
        && request.settings.search_endpoint == state.settings.search_endpoint;
    let key = if request.clear_search_api_key {
        None
    } else if request.search_api_key.is_some() {
        request.search_api_key
    } else if same_target {
        state.search_key.clone()
    } else {
        None
    };
    validate_key(&key)?;
    Ok((request.settings, key))
}
pub(super) fn validate_key(key: &Option<String>) -> Result<(), String> {
    if key.as_ref().is_some_and(|key| {
        key.trim().is_empty() || key.len() > 4096 || key.chars().any(char::is_control)
    }) {
        return Err("搜索 API 密钥无效".into());
    }
    Ok(())
}
pub(super) fn validate(settings: &AgentRuntimeSettings) -> Result<(), String> {
    if !matches!(settings.search_provider.as_str(), "brave" | "searxng") {
        return Err("搜索提供商须为 brave 或 searxng".into());
    }
    if !meowlive_adapters::search::valid_endpoint(&settings.search_endpoint) {
        return Err("搜索 API 地址格式无效".into());
    }
    let uri = validate_url(&settings.search_endpoint)?;
    let loopback = uri.host().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    if uri.scheme_str() != Some("https") && !(settings.search_provider == "searxng" && loopback) {
        return Err("搜索公网地址须使用 HTTPS，本地 SearXNG 可使用 HTTP 回环地址".into());
    }
    if !(1..=3).contains(&settings.max_tool_rounds)
        || !(1..=8).contains(&settings.tool_timeout_seconds)
    {
        return Err("工具轮数须为 1–3，工具超时须为 1–8 秒".into());
    }
    if settings.prices.len() > 64 {
        return Err("模型价格条目不能超过 64".into());
    }
    let mut seen = HashSet::new();
    for price in &settings.prices {
        if !text_valid(&price.provider, 64) || !text_valid(&price.model, 128) {
            return Err("模型价格的服务商或模型名无效".into());
        }
        validate_url(&price.base_url)?;
        if !seen.insert((
            &price.provider,
            price.base_url.trim_end_matches('/'),
            &price.model,
        )) {
            return Err("同一服务商、API 地址与模型的价格不能重复".into());
        }
        if [
            price.input_usd_per_million,
            price.output_usd_per_million,
            price.cache_read_usd_per_million,
            price.cache_write_usd_per_million,
        ]
        .iter()
        .any(|rate| !rate.is_finite() || !(0.0..=1_000_000.0).contains(rate))
        {
            return Err("模型价格须为有限的非负 USD/百万 token，最多 1000000".into());
        }
    }
    Ok(())
}
fn text_valid(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
pub(super) fn validate_url(value: &str) -> Result<axum::http::Uri, String> {
    let uri = value
        .parse::<axum::http::Uri>()
        .map_err(|_| "API 地址格式无效")?;
    if value.len() > 4096
        || value.contains(['#', '\\'])
        || value.chars().any(char::is_whitespace)
        || !matches!(uri.scheme_str(), Some("http" | "https"))
        || uri.host().is_none_or(str::is_empty)
        || uri
            .authority()
            .is_none_or(|authority| authority.as_str().contains('@'))
        || uri.query().is_some()
    {
        return Err("API 地址须为无凭据、查询参数或片段的 HTTP(S) 地址".into());
    }
    Ok(uri)
}
