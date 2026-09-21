//! Brave / SearXNG 的有界只读检索；端点只来自操作者设置，不由模型指定。
use futures_util::StreamExt;
use meowlive_application::ports::web_search::{SearchFuture, SearchResult, WebSearch};
use reqwest::{Client, Url};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};

#[derive(Clone, Copy)]
pub enum SearchProvider {
    Brave,
    Searxng,
}

pub struct HttpWebSearch {
    client: Client,
    provider: SearchProvider,
    endpoint: Url,
    key: Option<String>,
}

pub fn valid_endpoint(value: &str) -> bool {
    if value.len() > 2048 || value.chars().any(char::is_whitespace) {
        return false;
    }
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    let loopback = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    (url.scheme() == "https" || (url.scheme() == "http" && loopback))
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

impl HttpWebSearch {
    pub fn new(
        provider: SearchProvider,
        endpoint: &str,
        key: Option<String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        if !valid_endpoint(endpoint) || timeout.is_zero() || timeout > Duration::from_secs(8) {
            return Err("搜索端点或超时配置无效".into());
        }
        if matches!(provider, SearchProvider::Brave)
            && !key.as_ref().is_some_and(|key| {
                !key.is_empty() && key.len() <= 4096 && !key.chars().any(char::is_control)
            })
        {
            return Err("Brave 搜索尚未配置有效密钥".into());
        }
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .connect_timeout(timeout.min(Duration::from_secs(3)))
            .build()
            .map_err(|_| "无法创建搜索客户端")?;
        Ok(Self {
            client,
            provider,
            endpoint: Url::parse(endpoint).map_err(|_| "搜索地址无效")?,
            key,
        })
    }

    async fn fetch(&self, query: String) -> Result<Vec<SearchResult>, String> {
        if query.trim().is_empty()
            || query.chars().count() > 240
            || query.chars().any(char::is_control)
        {
            return Err("搜索词须为 1 至 240 个字符且不含控制字符".into());
        }
        let mut request = self
            .client
            .get(self.endpoint.clone())
            .header("accept", "application/json")
            .query(&[("q", query.trim())]);
        request = match self.provider {
            SearchProvider::Brave => request
                .header(
                    "x-subscription-token",
                    self.key.as_deref().unwrap_or_default(),
                )
                .query(&[
                    ("count", "5"),
                    ("result_filter", "web"),
                    ("text_decorations", "false"),
                ]),
            SearchProvider::Searxng => {
                request.query(&[("format", "json"), ("categories", "general")])
            }
        };
        let response = request.send().await.map_err(|_| "搜索连接失败或超时")?;
        if !response.status().is_success() {
            return Err(format!("搜索服务返回 HTTP {}", response.status().as_u16()));
        }
        const MAX_BYTES: usize = 262_144;
        if response
            .content_length()
            .is_some_and(|size| size > MAX_BYTES as u64)
        {
            return Err("搜索响应过大".into());
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| "搜索响应未完整接收")?;
            if bytes.len().saturating_add(chunk.len()) > MAX_BYTES {
                return Err("搜索响应过大".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let data: Value = serde_json::from_slice(&bytes)
            .map_err(|_| "搜索返回无效 JSON，请确认服务已启用 JSON 格式")?;
        let results = match self.provider {
            SearchProvider::Brave => data.pointer("/web/results"),
            SearchProvider::Searxng => data.get("results"),
        };
        // Brave omits web when a valid query has no matching pages.
        if results.is_none()
            && matches!(self.provider, SearchProvider::Brave)
            && data.get("query").is_some()
        {
            return Ok(vec![]);
        }
        let results = results
            .and_then(Value::as_array)
            .ok_or("搜索响应缺少结果列表")?;
        Ok(results
            .iter()
            .filter_map(|item| {
                let url = public_source(item.get("url")?.as_str()?)?;
                let title = plain(item.get("title")?.as_str()?, 200);
                if title.trim().is_empty() {
                    return None;
                }
                let snippet = plain(
                    item.get(match self.provider {
                        SearchProvider::Brave => "description",
                        SearchProvider::Searxng => "content",
                    })
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                    1000,
                );
                Some(SearchResult {
                    title,
                    url,
                    snippet,
                })
            })
            .take(5)
            .collect())
    }
}

impl WebSearch for HttpWebSearch {
    fn search(&self, query: String) -> SearchFuture<'_> {
        Box::pin(self.fetch(query))
    }
}

fn public_source(value: &str) -> Option<String> {
    if value.len() > 2048 || value.chars().any(char::is_control) {
        return None;
    }
    let url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = url.host_str()?.trim_matches(['[', ']']);
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || !host.contains(['.', ':'])
    {
        return None;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        let private = match ip {
            IpAddr::V4(ip) => {
                ip.is_private()
                    || ip.is_loopback()
                    || ip.is_link_local()
                    || ip.is_unspecified()
                    || ip.is_broadcast()
                    || ip.is_multicast()
            }
            IpAddr::V6(ip) => {
                ip.is_loopback()
                    || ip.is_unspecified()
                    || ip.is_unique_local()
                    || ip.is_unicast_link_local()
                    || ip.is_multicast()
                    || ip.to_ipv4_mapped().is_some()
            }
        };
        if private {
            return None;
        }
    }
    Some(url.to_string())
}

fn plain(value: &str, limit: usize) -> String {
    let mut in_tag = false;
    value
        .chars()
        .filter(|&c| {
            if c == '<' {
                in_tag = true;
                return false;
            }
            if c == '>' {
                in_tag = false;
                return false;
            }
            !in_tag && (!c.is_control() || c == '\n' || c == '\t')
        })
        .take(limit)
        .collect()
}
