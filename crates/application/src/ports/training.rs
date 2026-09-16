//! 训练持久化与受控进程端口；应用层不接收用户文件路径。
use meowlive_domain::training::{
    ArtifactPair, TrainingCatalog, TrainingClip, TrainingError, TrainingJob, TrainingState,
};
use std::{path::PathBuf, sync::atomic::AtomicBool};

#[derive(Clone, Debug)]
pub struct PreparedTrainingJob {
    pub job_file: PathBuf,
    pub work_dir: PathBuf,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedArtifactPair {
    pub gpt: PathBuf,
    pub sovits: PathBuf,
}
#[derive(Clone, Debug)]
pub struct TrainingProgress {
    pub state: TrainingState,
    pub progress: u8,
}

pub trait TrainingStore: Send + Sync {
    fn load(&self) -> Result<TrainingCatalog, TrainingError>;
    fn save(&self, catalog: &TrainingCatalog) -> Result<(), TrainingError>;
    fn new_job_id(&self) -> String;
    fn prepare(&self, job: &TrainingJob, clips: &[TrainingClip]) -> Result<(), TrainingError>;
    fn remove_unpublished(&self, id: &str) -> Result<(), TrainingError>;
    fn prepared(&self, id: &str) -> Result<PreparedTrainingJob, TrainingError>;
    fn resolve_pair(
        &self,
        id: &str,
        pair: &ArtifactPair,
    ) -> Result<ResolvedArtifactPair, TrainingError>;
}
pub trait TrainingEngine: Send + Sync {
    /// Returns only after the entire owned child tree has exited.
    fn run(
        &self,
        job: &PreparedTrainingJob,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(TrainingProgress),
    ) -> Result<ArtifactPair, TrainingError>;
}
