//! 时间筛选与按模型统计；历史扫描只保留最近 200 条，估算避免重复计算缓存和推理。
use super::{RuntimeStore, persistence};
use meowlive_protocol::llm_runtime::{
    LlmPrice, LlmTokenUsage, LlmUsageGroup, LlmUsageRecord, LlmUsageSnapshot, LlmUsageTotals,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageQuery {
    pub since_ms: Option<u64>,
    pub until_ms: Option<u64>,
    pub provider: Option<String>,
    pub model: Option<String>,
}
impl UsageQuery {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .since_ms
            .zip(self.until_ms)
            .is_some_and(|(since, until)| since > until)
            || self.provider.as_ref().is_some_and(|value| value.len() > 64)
            || self.model.as_ref().is_some_and(|value| value.len() > 128)
        {
            return Err("用量查询的时间范围或模型筛选无效".into());
        }
        Ok(())
    }
    fn matches(&self, record: &LlmUsageRecord) -> bool {
        self.since_ms
            .is_none_or(|since| record.started_at_ms >= since)
            && self
                .until_ms
                .is_none_or(|until| record.started_at_ms <= until)
            && self
                .provider
                .as_ref()
                .is_none_or(|provider| &record.provider == provider)
            && self
                .model
                .as_ref()
                .is_none_or(|model| &record.model == model)
    }
}

/// USD / million tokens multiplied by tokens gives micro USD. Reasoning is already
/// contained in output. Missing cache counters are only dispensable at equal rates.
pub fn estimate_cost(usage: &LlmTokenUsage, price: Option<&LlmPrice>) -> Option<u64> {
    let price = price?;
    let input = usage.input_tokens?;
    let output = usage.output_tokens?;
    if usage
        .reasoning_tokens
        .is_some_and(|reasoning| reasoning > output)
    {
        return None;
    }
    let read = match usage.cache_read_tokens {
        Some(tokens) => tokens,
        None if price.cache_read_usd_per_million == price.input_usd_per_million => 0,
        None => return None,
    };
    let write = match usage.cache_write_tokens {
        Some(tokens) => tokens,
        None if price.cache_write_usd_per_million == price.input_usd_per_million => 0,
        None => return None,
    };
    let ordinary = input.checked_sub(read.checked_add(write)?)?;
    let cost = (ordinary as f64) * price.input_usd_per_million
        + (read as f64) * price.cache_read_usd_per_million
        + (write as f64) * price.cache_write_usd_per_million
        + (output as f64) * price.output_usd_per_million;
    (cost.is_finite() && cost >= 0.0 && cost < (u64::MAX as f64)).then(|| cost.round() as u64)
}

pub(super) fn snapshot(store: &RuntimeStore, query: &UsageQuery) -> LlmUsageSnapshot {
    let (segments, pending, memory, prices, mut healthy, persistent) = {
        let inner = store.inner.lock().unwrap();
        (
            inner.disk.as_ref().map(|disk| disk.segments()).transpose(),
            inner.pending.values().cloned().collect::<Vec<_>>(),
            inner.memory_records.clone(),
            inner.settings.prices.clone(),
            inner.healthy,
            inner.disk.is_some(),
        )
    };
    let mut totals = LlmUsageTotals::default();
    let mut groups: BTreeMap<(String, String, String), LlmUsageTotals> = BTreeMap::new();
    let mut records = Vec::new();
    let mut collect = |mut record: LlmUsageRecord| -> Result<(), String> {
        if !query.matches(&record) {
            return Ok(());
        }
        let price = prices.iter().find(|price| {
            price.provider == record.provider
                && price.model == record.model
                && price.base_url.trim_end_matches('/') == record.base_url.trim_end_matches('/')
        });
        record.estimated_cost_microusd = estimate_cost(&record.usage, price);
        add(&mut totals, &record);
        add(
            groups
                .entry((
                    record.provider.clone(),
                    record.base_url.clone(),
                    record.model.clone(),
                ))
                .or_default(),
            &record,
        );
        let key = (record.started_at_ms, record.id.as_str());
        let position = records.partition_point(|other: &LlmUsageRecord| {
            (other.started_at_ms, other.id.as_str()) > key
        });
        if position < 200 {
            records.insert(position, record);
            records.truncate(200);
        }
        Ok(())
    };
    match segments {
        Ok(Some(segments)) => {
            if persistence::visit_segments(segments, &mut collect).is_err() {
                healthy = false;
            }
        }
        Err(_) => {
            healthy = false;
        }
        Ok(None) => {}
    }
    for record in memory.into_iter().chain(pending) {
        let _ = collect(record);
    }
    if !healthy {
        store.inner.lock().unwrap().healthy = false;
    }
    LlmUsageSnapshot {
        truncated: totals.calls > records.len() as u64 || !healthy,
        totals,
        groups: groups
            .into_iter()
            .map(|((provider, base_url, model), totals)| LlmUsageGroup {
                provider,
                base_url,
                model,
                totals,
            })
            .collect(),
        records,
        storage_available: persistent && healthy,
    }
}
fn add(totals: &mut LlmUsageTotals, record: &LlmUsageRecord) {
    totals.calls = totals.calls.saturating_add(1);
    totals.input_tokens = totals
        .input_tokens
        .saturating_add(record.usage.input_tokens.unwrap_or(0));
    totals.output_tokens = totals
        .output_tokens
        .saturating_add(record.usage.output_tokens.unwrap_or(0));
    totals.cache_read_tokens = totals
        .cache_read_tokens
        .saturating_add(record.usage.cache_read_tokens.unwrap_or(0));
    totals.cache_write_tokens = totals
        .cache_write_tokens
        .saturating_add(record.usage.cache_write_tokens.unwrap_or(0));
    totals.reasoning_tokens = totals
        .reasoning_tokens
        .saturating_add(record.usage.reasoning_tokens.unwrap_or(0));
    match record.estimated_cost_microusd {
        Some(cost) => {
            totals.estimated_cost_microusd = totals.estimated_cost_microusd.saturating_add(cost)
        }
        None => totals.unpriced_calls = totals.unpriced_calls.saturating_add(1),
    }
    if record.usage.input_tokens.is_none() || record.usage.output_tokens.is_none() {
        totals.unknown_usage_calls = totals.unknown_usage_calls.saturating_add(1);
    }
}
