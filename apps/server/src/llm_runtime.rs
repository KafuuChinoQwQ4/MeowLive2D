//! Agent 运行配置、实时活动与独立模型调用账本；不存储提示词、工具正文或上游错误。
mod ledger;
mod metering;
mod persistence;
mod settings;

pub use ledger::{UsageQuery, estimate_cost};
use meowlive_protocol::llm_runtime::{
    AgentActivitySnapshot, AgentRuntimeSettings, AgentRuntimeSettingsRequest,
    AgentRuntimeSettingsSnapshot, LlmUsageRecord, LlmUsageSnapshot,
};
pub use metering::measured_turn;
use persistence::DiskLedger;
use std::{collections::BTreeMap, path::Path, sync::Mutex};

pub struct RuntimeStore {
    inner: Mutex<RuntimeState>,
}
struct RuntimeState {
    settings: AgentRuntimeSettings,
    search_key: Option<String>,
    activity: AgentActivitySnapshot,
    pending: BTreeMap<String, LlmUsageRecord>,
    // In-memory stores are for tests. Production history is streamed from bounded segments.
    memory_records: Vec<LlmUsageRecord>,
    disk: Option<DiskLedger>,
    healthy: bool,
}

impl RuntimeStore {
    pub fn memory() -> Self {
        Self {
            inner: Mutex::new(RuntimeState {
                settings: settings::defaults(),
                search_key: None,
                activity: AgentActivitySnapshot {
                    run_id: None,
                    phase: "idle".into(),
                    started_at_ms: None,
                    updated_at_ms: now_ms(),
                    output_characters: 0,
                    tool_round: 0,
                    tools: vec![],
                    message: String::new(),
                },
                pending: BTreeMap::new(),
                memory_records: Vec::new(),
                disk: None,
                healthy: true,
            }),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let (mut disk, saved, mut pending) = DiskLedger::open(path.as_ref())?;
        // If a crash occurred between final append and pending replacement, the appended
        // record wins. Otherwise the provider's outcome is unknown after a process restart.
        disk.visit(|record| {
            pending.remove(&record.id);
            Ok(())
        })?;
        for record in pending.values_mut() {
            record.status = "interrupted".into();
            disk.append(record)?;
        }
        disk.save_pending(&BTreeMap::new())?;
        let store = Self::memory();
        {
            let mut inner = store.inner.lock().unwrap();
            if let Some(saved) = saved {
                inner.settings = saved.settings;
                inner.search_key = saved.search_api_key;
            }
            inner.disk = Some(disk);
        }
        Ok(store)
    }

    pub fn settings(&self) -> AgentRuntimeSettings {
        self.inner.lock().unwrap().settings.clone()
    }
    /// Retrieve the credential together with the destination it belongs to.
    pub fn settings_with_search_key(&self) -> (AgentRuntimeSettings, Option<String>) {
        let inner = self.inner.lock().unwrap();
        (inner.settings.clone(), inner.search_key.clone())
    }
    pub fn search_key(&self) -> Option<String> {
        self.inner.lock().unwrap().search_key.clone()
    }
    pub fn snapshot(&self) -> AgentRuntimeSettingsSnapshot {
        let inner = self.inner.lock().unwrap();
        settings::snapshot(&inner)
    }
    pub fn save(
        &self,
        request: AgentRuntimeSettingsRequest,
    ) -> Result<AgentRuntimeSettingsSnapshot, String> {
        let mut inner = self.inner.lock().unwrap();
        let (settings, key) = settings::resolve(&inner, request)?;
        if let Some(disk) = &inner.disk {
            if let Err(error) = disk.save_settings(&settings, &key) {
                inner.healthy = false;
                return Err(error);
            }
        }
        inner.settings = settings;
        inner.search_key = key;
        Ok(settings::snapshot(&inner))
    }
    pub fn activity(&self) -> AgentActivitySnapshot {
        self.inner.lock().unwrap().activity.clone()
    }
    pub fn set_activity(&self, activity: AgentActivitySnapshot) {
        self.inner.lock().unwrap().activity = activity;
    }
    /// Check ownership and apply progress under the same lock so a late event from
    /// an older turn cannot overwrite a newer run's activity.
    pub fn update_activity_for(
        &self,
        run_id: &str,
        update: impl FnOnce(&mut AgentActivitySnapshot),
    ) {
        let mut inner = self.inner.lock().unwrap();
        if inner.activity.run_id.as_deref() == Some(run_id) {
            update(&mut inner.activity);
        }
    }
    /// Potentially scans disk history. HTTP callers execute this in spawn_blocking.
    pub fn usage(&self, query: &UsageQuery) -> LlmUsageSnapshot {
        ledger::snapshot(self, query)
    }
}

pub(super) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}
