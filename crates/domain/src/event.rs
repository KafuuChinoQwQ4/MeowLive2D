//! 平台无关的直播事件；身份字段为可打印 ASCII，内容按 Unicode 字符计数。

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveEvent {
    /// Globally unique event identity: 1–128 printable ASCII characters.
    pub id: String,
    /// Platform identity: 1–32 printable ASCII characters.
    pub source: String,
    pub viewer: String,
    /// Stable platform identity when the source provided one.
    pub viewer_identity: Option<ViewerIdentity>,
    /// UTC milliseconds supplied by the event source.
    pub occurred_at_ms: u64,
    /// Raw gift fields supplied by the source; never a calculated total.
    pub gift_metadata: Option<GiftMetadata>,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerIdentity {
    pub namespace: String,
    pub kind: ViewerIdentityKind,
    pub external_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewerIdentityKind {
    OpenId,
    Uid,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GiftMetadata {
    pub price: Option<u64>,
    pub paid: Option<bool>,
    pub medal_level: Option<u32>,
    pub guard_level: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    Chat {
        text: String,
    },
    Gift {
        name: String,
        count: u32,
    },
    SuperChat {
        text: String,
        amount_cny: u32,
        start_at_ms: u64,
        end_at_ms: u64,
    },
    RoomEnter,
}

impl LiveEvent {
    pub fn validate(&self) -> Result<(), String> {
        identity("event id", &self.id, 128)?;
        identity("event source", &self.source, 32)?;
        content("viewer", &self.viewer, 64, false)?;
        if let Some(viewer_identity) = &self.viewer_identity {
            viewer_identity.validate()?;
        }
        if self.gift_metadata.is_some() && !matches!(self.kind, EventKind::Gift { .. }) {
            return Err("gift metadata requires a gift event".into());
        }
        match &self.kind {
            EventKind::Chat { text } => content("chat text", text, 500, true),
            EventKind::Gift { name, count } => {
                content("gift name", name, 100, false)?;
                if !(1..=10_000).contains(count) {
                    return Err("gift count must be between 1 and 10000".into());
                }
                Ok(())
            }
            EventKind::SuperChat {
                text,
                amount_cny,
                start_at_ms,
                end_at_ms,
            } => {
                content("super chat text", text, 500, true)?;
                if !(1..=1_000_000).contains(amount_cny) {
                    return Err("super chat amount must be between 1 and 1000000 CNY".into());
                }
                if end_at_ms <= start_at_ms || *end_at_ms > 9_007_199_254_740_991 {
                    return Err(
                        "super chat display times must be ordered safe millisecond timestamps"
                            .into(),
                    );
                }
                Ok(())
            }
            EventKind::RoomEnter => Ok(()),
        }
    }

    /// Source-relative lifetime; caller preserves elapsed source age when scheduling.
    pub fn response_ttl_ms(&self, ordinary_ttl_ms: u64) -> u64 {
        match self.kind {
            EventKind::SuperChat {
                start_at_ms,
                end_at_ms,
                ..
            } => end_at_ms.saturating_sub(start_at_ms),
            EventKind::RoomEnter => ordinary_ttl_ms.min(15_000),
            _ => ordinary_ttl_ms,
        }
    }
}

impl ViewerIdentity {
    pub fn validate(&self) -> Result<(), String> {
        identity("viewer identity namespace", &self.namespace, 128)?;
        identity("viewer identity external id", &self.external_id, 128)?;
        if matches!(self.kind, ViewerIdentityKind::Uid) {
            let uid = self
                .external_id
                .parse::<u64>()
                .map_err(|_| "viewer UID must be a positive integer".to_string())?;
            if uid == 0 || uid.to_string() != self.external_id {
                return Err("viewer UID must be a positive integer".into());
            }
        }
        Ok(())
    }
}

fn identity(label: &str, value: &str, maximum: usize) -> Result<(), String> {
    content(label, value, maximum, false)?;
    if !value.bytes().all(|byte| (b' '..=b'~').contains(&byte)) {
        return Err(format!("{label} must contain printable ASCII only"));
    }
    Ok(())
}

fn content(label: &str, value: &str, maximum: usize, multiline: bool) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().count() > maximum {
        return Err(format!("{label} must contain 1 to {maximum} characters"));
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !(multiline && matches!(ch, '\n' | '\t')))
    {
        return Err(format!("{label} contains a control character"));
    }
    Ok(())
}
