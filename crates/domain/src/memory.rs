//! 可核对来源的记忆候选与保守生命周期规则；时间单位为 Unix 毫秒。
use std::collections::BTreeSet;
pub const DAY_MS: i64 = 86_400_000;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySource {
    pub source: String,
    pub viewer_id: String,
    pub event_id: String,
    pub text: String,
    pub occurred_at_ms: i64,
    pub day: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub source: String,
    pub event_id: String,
    pub quote: String,
    pub occurred_at_ms: i64,
    pub day: i64,
}
impl Evidence {
    pub fn from_source(s: &MemorySource, quote: &str) -> Self {
        Self {
            source: s.source.clone(),
            event_id: s.event_id.clone(),
            quote: quote.into(),
            occurred_at_ms: s.occurred_at_ms,
            day: s.day,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryKind {
    Preference,
    PreferredName,
    StableFact,
    Experience,
    TemporaryState,
    ThirdPartyClaim,
    SensitiveInference,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryStatus {
    Candidate,
    ShortTerm,
    LongTerm,
    Expired,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MemoryCandidate {
    pub key: String,
    pub value: String,
    pub kind: MemoryKind,
    pub evidence: Vec<Evidence>,
    pub explicit: bool,
    pub confidence: f64,
    pub valid_until_ms: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAssessment {
    pub status: MemoryStatus,
    pub expires_at_ms: Option<i64>,
    pub evidence_count: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryValidationError;
impl std::fmt::Display for MemoryValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid memory evidence")
    }
}
impl std::error::Error for MemoryValidationError {}
pub fn short_term_weight(age_ms: i64) -> f64 {
    2_f64.powf(-(age_ms.max(0) as f64) / (7 * DAY_MS) as f64)
}
pub fn assess(
    c: &MemoryCandidate,
    sources: &[MemorySource],
    now: i64,
) -> Result<MemoryAssessment, MemoryValidationError> {
    let err = MemoryValidationError;
    let key = match c.kind {
        MemoryKind::Preference => "preference",
        MemoryKind::PreferredName => "preferred_name",
        MemoryKind::StableFact => "stable_fact",
        MemoryKind::Experience => "experience",
        MemoryKind::TemporaryState => "temporary_state",
        MemoryKind::ThirdPartyClaim => "third_party_claim",
        MemoryKind::SensitiveInference => "sensitive_inference",
    };
    if c.key != key
        || c.value.trim().is_empty()
        || c.value.len() > 512
        || !c.confidence.is_finite()
        || !(0.0..=1.0).contains(&c.confidence)
        || c.evidence.is_empty()
        || c.evidence.len() > 32
        || sources.is_empty()
        || sources.len() > 32
    {
        return Err(err);
    }
    let viewer = &sources[0].viewer_id;
    if viewer.is_empty()
        || sources.iter().any(|s| {
            s.viewer_id != *viewer
                || s.event_id.is_empty()
                || s.source.is_empty()
                || s.text.len() > 8192
                || s.occurred_at_ms < 0
                || s.occurred_at_ms > now
        })
    {
        return Err(err);
    }
    let mut ids = BTreeSet::new();
    let mut days = BTreeSet::new();
    let mut first = now;
    let mut clear = true;
    let mut self_report = true;
    for e in &c.evidence {
        let matches: Vec<_> = sources
            .iter()
            .filter(|s| s.source == e.source && s.event_id == e.event_id)
            .collect();
        if matches.len() != 1 {
            return Err(err);
        }
        let s = matches[0];
        if e.quote.trim().is_empty()
            || e.quote.len() > 2048
            || !s.text.contains(&e.quote)
            || !e.quote.contains(&c.value)
            || e.day != s.day
            || e.occurred_at_ms != s.occurred_at_ms
        {
            return Err(err);
        }
        first = first.min(s.occurred_at_ms);
        self_report &= s.text.trim() == e.quote
            && match c.kind {
                MemoryKind::Preference => [
                    format!("我比较喜欢{}", c.value),
                    format!("我觉得{}不错", c.value),
                    format!("我喜欢{}", c.value),
                ]
                .contains(&e.quote),
                MemoryKind::PreferredName => [
                    format!("我一般被叫作{}", c.value),
                    format!("我的昵称是{}", c.value),
                ]
                .contains(&e.quote),
                MemoryKind::StableFact => {
                    [format!("我养{}", c.value), format!("我的职业是{}", c.value)]
                        .contains(&e.quote)
                }
                MemoryKind::Experience => [
                    format!("我昨天去了{}", c.value),
                    format!("我今天去了{}", c.value),
                    format!("我参加了{}", c.value),
                ]
                .contains(&e.quote),
                MemoryKind::TemporaryState => [
                    format!("我今天很{}", c.value),
                    format!("我现在很{}", c.value),
                ]
                .contains(&e.quote),
                _ => false,
            }
            && ![
                "如果",
                "假如",
                "可能",
                "也许",
                "听说",
                "他说",
                "她说",
                "不是",
                "不喜欢",
                "讨厌",
                "开玩笑",
                "骗",
                "抑郁",
                "疾病",
                "政治",
                "宗教",
                "性取向",
                "诊断",
            ]
            .iter()
            .any(|w| e.quote.contains(w));
        if ids.insert((&e.source, &e.event_id)) {
            days.insert(s.day);
        }
        let exact = match c.kind {
            MemoryKind::Preference => {
                [format!("我喜欢{}", c.value), format!("我最喜欢{}", c.value)]
                    .iter()
                    .any(|p| e.quote == *p)
            }
            MemoryKind::PreferredName => [format!("请叫我{}", c.value), format!("叫我{}", c.value)]
                .iter()
                .any(|p| e.quote == *p),
            _ => false,
        };
        clear &= exact
            && s.text.trim() == e.quote
            && ![
                "如果",
                "假如",
                "可能",
                "也许",
                "听说",
                "开玩笑",
                "骗",
                "抑郁",
                "疾病",
                "政治",
                "宗教",
                "性取向",
                "诊断",
                "但",
                "不",
                "，",
                ",",
                "。",
                ";",
                "；",
            ]
            .iter()
            .any(|w| c.value.contains(w));
    }
    let mut status = match c.kind {
        MemoryKind::Preference | MemoryKind::PreferredName if clear => MemoryStatus::LongTerm,
        MemoryKind::Preference | MemoryKind::PreferredName | MemoryKind::StableFact
            if days.len() >= 2 && self_report =>
        {
            MemoryStatus::LongTerm
        }
        MemoryKind::Experience | MemoryKind::TemporaryState if self_report => {
            MemoryStatus::ShortTerm
        }
        _ => MemoryStatus::Candidate,
    };
    let ttl = match status {
        MemoryStatus::LongTerm => None,
        MemoryStatus::ShortTerm if c.kind == MemoryKind::TemporaryState => Some(DAY_MS),
        MemoryStatus::ShortTerm => Some(30 * DAY_MS),
        _ => Some(7 * DAY_MS),
    };
    let mut expires = ttl.map(|t| first.saturating_add(t));
    if let Some(until) = c.valid_until_ms {
        if until < first {
            return Err(err);
        }
        expires = Some(expires.map_or(until, |v| v.min(until)));
    }
    if expires.is_some_and(|t| t <= now) {
        status = MemoryStatus::Expired
    }
    Ok(MemoryAssessment {
        status,
        expires_at_ms: expires,
        evidence_count: ids.len(),
    })
}
