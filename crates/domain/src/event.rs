//! 平台无关的直播事件；身份字段为可打印 ASCII，内容按 Unicode 字符计数。

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveEvent {
    /// Globally unique event identity: 1–128 printable ASCII characters.
    pub id: String,
    /// Platform identity: 1–32 printable ASCII characters.
    pub source: String,
    pub viewer: String,
    pub occurred_at_ms: u64,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    Chat { text: String },
    Gift { name: String, count: u32 },
}

impl LiveEvent {
    pub fn validate(&self) -> Result<(), String> {
        identity("event id", &self.id, 128)?;
        identity("event source", &self.source, 32)?;
        content("viewer", &self.viewer, 64, false)?;
        match &self.kind {
            EventKind::Chat { text } => content("chat text", text, 500, true),
            EventKind::Gift { name, count } => {
                content("gift name", name, 100, false)?;
                if !(1..=10_000).contains(count) {
                    return Err("gift count must be between 1 and 10000".into());
                }
                Ok(())
            }
        }
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
