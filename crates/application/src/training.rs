//! 单训练互斥、持久化状态与试听后保存、激活的训练用例。
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_service_is_explicit_and_cannot_queue() {
        let service = TrainingManager::disabled();
        assert!(!service.snapshot().configured);
        assert!(matches!(
            service.create(
                TrainingParameters {
                    name: "测试".into(),
                    voice_id: "default".into(),
                    sovits_epochs: 1,
                    gpt_epochs: 1,
                    performance: TrainingPerformance::default(),
                },
                vec![]
            ),
            Err(TrainingError::Disabled)
        ));
    }
}

use crate::ports::training::{
    ResolvedArtifactPair, TrainingEngine, TrainingProgress, TrainingStore,
};
use meowlive_domain::training::*;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct TrainingSnapshot {
    pub configured: bool,
    pub busy: bool,
    pub jobs: Vec<TrainingJob>,
    pub active_versions: std::collections::BTreeMap<String, String>,
}
struct State {
    catalog: TrainingCatalog,
    running: Option<String>,
    transcribing: bool,
    closing: bool,
}
enum VersionChange {
    Audition,
    Save,
    Activate,
}
pub struct TrainingManager {
    store: Option<Arc<dyn TrainingStore>>,
    engine: Option<Arc<dyn TrainingEngine>>,
    state: Mutex<State>,
    completed: Condvar,
    cancelled: AtomicBool,
}
impl TrainingManager {
    pub fn disabled() -> Self {
        Self {
            store: None,
            engine: None,
            state: Mutex::new(State {
                catalog: TrainingCatalog::default(),
                running: None,
                transcribing: false,
                closing: false,
            }),
            completed: Condvar::new(),
            cancelled: AtomicBool::new(false),
        }
    }
    pub fn open(
        store: Arc<dyn TrainingStore>,
        engine: Arc<dyn TrainingEngine>,
    ) -> Result<Self, TrainingError> {
        let mut catalog = store.load()?;
        catalog.validate()?;
        let mut changed = false;
        for job in &mut catalog.jobs {
            if !job.state.is_terminal() {
                job.state = TrainingState::Interrupted;
                job.message = "服务重启，训练已中断，请新建任务".into();
                job.updated_at_ms = now_ms();
                changed = true;
            }
        }
        if changed {
            store.save(&catalog)?;
        }
        Ok(Self {
            store: Some(store),
            engine: Some(engine),
            state: Mutex::new(State {
                catalog,
                running: None,
                transcribing: false,
                closing: false,
            }),
            completed: Condvar::new(),
            cancelled: AtomicBool::new(false),
        })
    }
    pub fn snapshot(&self) -> TrainingSnapshot {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        TrainingSnapshot {
            configured: self.store.is_some(),
            busy: (state.running.is_some() || state.transcribing)
                || state
                    .catalog
                    .jobs
                    .iter()
                    .any(|job| !job.state.is_terminal()),
            jobs: state.catalog.jobs.clone(),
            active_versions: state.catalog.active_versions.clone(),
        }
    }
    /// Call from a blocking worker; the slot remains owned if the HTTP request ends.
    pub fn transcribe(&self, clip: TrainingClip) -> Result<String, TrainingError> {
        let engine = self.engine.as_ref().ok_or(TrainingError::Disabled)?;
        clip.validate()?;
        if !clip.text.is_empty() {
            return Err(TrainingError::Invalid(
                "自动提取文本只接收音频和语言".into(),
            ));
        }
        {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.closing
                || state.running.is_some()
                || state.transcribing
                || state
                    .catalog
                    .jobs
                    .iter()
                    .any(|job| !job.state.is_terminal())
            {
                return Err(TrainingError::Busy);
            }
            state.transcribing = true;
            self.cancelled.store(false, Ordering::SeqCst);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let text = engine.transcribe(&clip, &self.cancelled)?;
            if self.cancelled.load(Ordering::SeqCst) {
                return Err(TrainingError::Cancelled);
            }
            let checked = TrainingClip { text, ..clip };
            if checked.text.is_empty() || checked.validate().is_err() {
                return Err(TrainingError::Engine(
                    "自动提取未得到有效文本，请重新选择音频或手动填写文本".into(),
                ));
            }
            Ok(checked.text)
        }))
        .unwrap_or_else(|_| {
            Err(TrainingError::Engine(
                "自动提取文本失败，请手动填写文本后重试".into(),
            ))
        });
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.transcribing = false;
        self.completed.notify_all();
        result
    }
    pub fn create(
        &self,
        parameters: TrainingParameters,
        clips: Vec<TrainingClip>,
    ) -> Result<TrainingJob, TrainingError> {
        let store = self.store.as_ref().ok_or(TrainingError::Disabled)?;
        parameters.validate()?;
        if !(2..=MAX_TRAINING_CLIPS).contains(&clips.len()) {
            return Err(TrainingError::Invalid("请提供 2 至 32 个人工切片".into()));
        }
        let automatic = clips[0].text.is_empty();
        if clips.iter().any(|clip| clip.text.is_empty() != automatic) {
            return Err(TrainingError::Invalid(
                "请为全部片段提供文本，或全部留空自动识别".into(),
            ));
        }
        let mut total_bytes = 0usize;
        for clip in &clips {
            clip.validate()?;
            total_bytes = total_bytes.saturating_add(clip.wav.len());
        }
        if total_bytes > MAX_DATASET_BYTES {
            return Err(TrainingError::Invalid("训练数据总量不能超过 32 MiB".into()));
        }
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.closing
            || (state.running.is_some() || state.transcribing)
            || state
                .catalog
                .jobs
                .iter()
                .any(|job| !job.state.is_terminal())
        {
            return Err(TrainingError::Busy);
        }
        if state.catalog.jobs.len() >= MAX_TRAINING_JOBS {
            return Err(TrainingError::Capacity);
        }
        let now = now_ms();
        let job = TrainingJob {
            id: store.new_job_id(),
            parameters,
            state: TrainingState::Queued,
            clip_count: clips.len(),
            total_bytes,
            created_at_ms: now,
            updated_at_ms: now,
            progress: 0,
            message: "已排队".into(),
            artifacts: None,
            auditioned: false,
            saved: false,
        };
        let mut next = state.catalog.clone();
        next.jobs.push(job.clone());
        next.validate()?;
        let base = state
            .catalog
            .jobs
            .iter()
            .filter(|previous| {
                previous.parameters.voice_id == job.parameters.voice_id
                    && previous.state == TrainingState::Succeeded
            })
            .max_by_key(|previous| previous.created_at_ms);
        store.prepare(&job, &clips, base)?;
        if let Err(error) = store.save(&next) {
            store.remove_unpublished(&job.id)?;
            return Err(error);
        }
        state.catalog = next;
        Ok(job)
    }
    /// Call from a blocking worker. Cancellation never frees the slot until this returns.
    pub fn run(&self, id: &str) -> Result<TrainingJob, TrainingError> {
        let store = self.store.as_ref().ok_or(TrainingError::Disabled)?;
        let engine = self.engine.as_ref().ok_or(TrainingError::Disabled)?;
        {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            if (state.running.is_some() || state.transcribing) || state.closing {
                return Err(TrainingError::Busy);
            }
            let mut next = state.catalog.clone();
            let job = next
                .jobs
                .iter_mut()
                .find(|job| job.id == id)
                .ok_or(TrainingError::NotFound)?;
            if job.state != TrainingState::Queued {
                return Err(TrainingError::Invalid("任务不能重复启动".into()));
            }
            job.state = TrainingState::Preparing;
            job.message = "准备训练数据".into();
            job.updated_at_ms = now_ms();
            if let Err(error) = store.save(&next) {
                // No child has started yet. Even when storage remains unavailable,
                // publish a terminal in-memory state instead of stranding the queue.
                // The last durable queued state becomes interrupted on restart.
                let job = next
                    .jobs
                    .iter_mut()
                    .find(|job| job.id == id)
                    .ok_or(TrainingError::NotFound)?;
                job.state = TrainingState::Failed;
                job.message = "训练启动失败，无法保存启动状态，请检查训练存储".into();
                job.updated_at_ms = now_ms();
                let _ = store.save(&next);
                state.catalog = next;
                return Err(error);
            }
            state.catalog = next;
            state.running = Some(id.into());
            self.cancelled.store(false, Ordering::SeqCst);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let prepared = store.prepared(id)?;
            let mut progress_error = None;
            let mut callback = |progress: TrainingProgress| {
                if let Err(error) = self.update_progress(id, progress) {
                    progress_error = Some(error);
                    self.cancelled.store(true, Ordering::SeqCst);
                }
            };
            let pair = engine.run(&prepared, &self.cancelled, &mut callback)?;
            if let Some(error) = progress_error {
                return Err(error);
            }
            pair.validate()?;
            store.resolve_pair(id, &pair)?;
            Ok(pair)
        }))
        .unwrap_or_else(|_| Err(TrainingError::Engine("训练进程异常退出".into())));
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let mut next = state.catalog.clone();
        let job = next
            .jobs
            .iter_mut()
            .find(|job| job.id == id)
            .ok_or(TrainingError::NotFound)?;
        if self.cancelled.load(Ordering::SeqCst) || matches!(result, Err(TrainingError::Cancelled))
        {
            job.state = TrainingState::Cancelled;
            job.message = "训练已取消，子进程已退出".into();
        } else {
            match result {
                Ok(pair) => {
                    job.state = TrainingState::Succeeded;
                    job.artifacts = Some(pair);
                    job.progress = 100;
                    job.message = "训练完成，请试听后保存音色".into();
                }
                Err(TrainingError::Engine(message)) => {
                    job.state = TrainingState::Failed;
                    job.message = message;
                }
                Err(_) => {
                    job.state = TrainingState::Failed;
                    job.message = "训练失败，请检查本地引擎配置及运行日志".into();
                }
            }
        }
        job.updated_at_ms = now_ms();
        let job = job.clone();
        // A failed disk write cannot leave a phantom running process in memory. The persisted
        // nonterminal state is conservatively changed to interrupted on the next open.
        let saved = store.save(&next);
        state.catalog = next;
        state.running = None;
        self.completed.notify_all();
        saved?;
        Ok(job)
    }
    pub fn cancel(&self, id: &str) -> Result<TrainingJob, TrainingError> {
        let store = self.store.as_ref().ok_or(TrainingError::Disabled)?;
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let mut next = state.catalog.clone();
        let job = next
            .jobs
            .iter_mut()
            .find(|job| job.id == id)
            .ok_or(TrainingError::NotFound)?;
        if job.state.is_terminal() {
            return Ok(job.clone());
        }
        if state.running.as_deref() == Some(id) {
            self.cancelled.store(true, Ordering::SeqCst);
            job.state = TrainingState::Cancelling;
            job.message = "正在终止训练进程".into();
        } else {
            job.state = TrainingState::Cancelled;
            job.message = "排队任务已取消".into();
        }
        job.updated_at_ms = now_ms();
        let job = job.clone();
        store.save(&next)?;
        state.catalog = next;
        Ok(job)
    }
    pub fn mark_auditioned(&self, id: &str) -> Result<TrainingJob, TrainingError> {
        self.change_version(id, VersionChange::Audition)
    }
    pub fn save_version(&self, id: &str) -> Result<TrainingJob, TrainingError> {
        self.change_version(id, VersionChange::Save)
    }
    pub fn activate(&self, id: &str) -> Result<TrainingJob, TrainingError> {
        self.change_version(id, VersionChange::Activate)
    }
    /// Check deletion admission before committing related resource changes.
    pub fn check_delete(&self, id: &str) -> Result<(), TrainingError> {
        self.store.as_ref().ok_or(TrainingError::Disabled)?;
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        Self::validate_deletion(&state, id)
    }
    fn validate_deletion(state: &State, id: &str) -> Result<(), TrainingError> {
        if !valid_job_id(id) {
            return Err(TrainingError::Invalid("训练任务标识无效".into()));
        }
        if (state.running.is_some() || state.transcribing)
            || state.closing
            || state
                .catalog
                .jobs
                .iter()
                .any(|job| !job.state.is_terminal())
        {
            return Err(TrainingError::Busy);
        }
        Ok(())
    }
    /// Delete a terminal training task and its owned model directory.
    /// Catalog state is committed first, so a cleanup failure cannot leave a
    /// visible version pointing at a removed or partially removed catalog entry.
    pub fn delete(&self, id: &str) -> Result<(), TrainingError> {
        let store = self.store.as_ref().ok_or(TrainingError::Disabled)?;
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        Self::validate_deletion(&state, id)?;
        if !state.catalog.jobs.iter().any(|job| job.id == id) {
            // A previous call may have committed the catalog and failed only
            // during filesystem cleanup. Retrying is intentionally idempotent.
            return store.remove_published(id);
        }
        let mut next = state.catalog.clone();
        next.jobs.retain(|candidate| candidate.id != id);
        next.active_versions.retain(|_, active| active != id);
        next.validate()?;
        store.save(&next)?;
        state.catalog = next;
        store
            .remove_published(id)
            .map_err(|_| TrainingError::Store("记录已删除，但文件清理失败，请重试删除".into()))
    }
    fn change_version(
        &self,
        id: &str,
        change: VersionChange,
    ) -> Result<TrainingJob, TrainingError> {
        let store = self.store.as_ref().ok_or(TrainingError::Disabled)?;
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.closing
            || (state.running.is_some() || state.transcribing)
            || state
                .catalog
                .jobs
                .iter()
                .any(|job| !job.state.is_terminal())
        {
            return Err(TrainingError::Busy);
        }
        let mut next = state.catalog.clone();
        let job = next
            .jobs
            .iter_mut()
            .find(|job| job.id == id)
            .ok_or(TrainingError::NotFound)?;
        let pair = job
            .artifacts
            .as_ref()
            .filter(|_| job.state == TrainingState::Succeeded)
            .ok_or(TrainingError::Invalid("任务尚未生成完整模型".into()))?;
        store.resolve_pair(id, pair)?;
        match change {
            VersionChange::Audition => job.auditioned = true,
            VersionChange::Save | VersionChange::Activate => {
                if !job.auditioned {
                    return Err(TrainingError::NotAuditioned);
                }
                if matches!(change, VersionChange::Save) && job.saved {
                    return Ok(job.clone());
                }
                job.saved = true;
                if matches!(change, VersionChange::Activate) {
                    next.active_versions
                        .insert(job.parameters.voice_id.clone(), id.into());
                }
            }
        }
        job.updated_at_ms = now_ms();
        let job = job.clone();
        next.validate()?;
        store.save(&next)?;
        state.catalog = next;
        Ok(job)
    }
    pub fn resolve_version(&self, id: &str) -> Result<ResolvedArtifactPair, TrainingError> {
        let store = self.store.as_ref().ok_or(TrainingError::Disabled)?;
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let job = state
            .catalog
            .jobs
            .iter()
            .find(|job| job.id == id)
            .ok_or(TrainingError::NotFound)?;
        let pair = job
            .artifacts
            .as_ref()
            .filter(|_| job.state == TrainingState::Succeeded)
            .ok_or(TrainingError::Invalid("任务尚未生成完整模型".into()))?;
        store.resolve_pair(id, pair)
    }
    pub fn resolve_active_pair(
        &self,
        voice_id: &str,
    ) -> Result<Option<ResolvedArtifactPair>, TrainingError> {
        let snapshot = self.snapshot();
        snapshot
            .active_versions
            .get(voice_id)
            .map(|id| self.resolve_version(id))
            .transpose()
    }
    pub fn shutdown(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.closing = true;
        self.cancelled.store(true, Ordering::SeqCst);
        while state.running.is_some() || state.transcribing {
            state = self
                .completed
                .wait(state)
                .unwrap_or_else(|p| p.into_inner());
        }
    }
    fn update_progress(&self, id: &str, progress: TrainingProgress) -> Result<(), TrainingError> {
        if !matches!(
            progress.state,
            TrainingState::Preparing | TrainingState::Training | TrainingState::Validating
        ) {
            return Ok(());
        }
        let store = self.store.as_ref().ok_or(TrainingError::Disabled)?;
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let mut next = state.catalog.clone();
        let job = next
            .jobs
            .iter_mut()
            .find(|job| job.id == id)
            .ok_or(TrainingError::NotFound)?;
        if self.cancelled.load(Ordering::SeqCst) || job.state.is_terminal() {
            return Ok(());
        }
        let rank = |stage| match stage {
            TrainingState::Preparing => 0,
            TrainingState::Training => 1,
            TrainingState::Validating => 2,
            _ => 3,
        };
        if rank(progress.state) < rank(job.state) {
            return Ok(());
        }
        job.state = progress.state;
        job.progress = job.progress.max(progress.progress.min(99));
        job.updated_at_ms = now_ms();
        job.message = match job.state {
            TrainingState::Preparing => "准备训练数据",
            TrainingState::Training => "正在训练模型",
            _ => "正在验证成对模型",
        }
        .into();
        store.save(&next)?;
        state.catalog = next;
        Ok(())
    }
}
impl Drop for TrainingManager {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}
