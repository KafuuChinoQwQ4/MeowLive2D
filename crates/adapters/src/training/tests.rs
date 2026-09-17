use super::*;
use meowlive_application::{
    ports::training::{PreparedTrainingJob, TrainingEngine, TrainingProgress, TrainingStore},
    training::TrainingManager,
};
use meowlive_domain::training::*;
use std::{
    fs,
    io::Cursor,
    sync::{Arc, atomic::AtomicBool},
};
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("meowlive-training-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn clip() -> TrainingClip {
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(
            &mut bytes,
            hound::WavSpec {
                channels: 1,
                sample_rate: 8000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..24000 {
            writer.write_sample(1000i16).unwrap();
        }
        writer.finalize().unwrap();
    }
    TrainingClip {
        text: "这是人工确认的文本。".into(),
        language: "zh".into(),
        wav: bytes.into_inner(),
    }
}
fn parameters() -> TrainingParameters {
    TrainingParameters {
        name: "测试版本".into(),
        voice_id: "default".into(),
        sovits_epochs: 1,
        gpt_epochs: 1,
        performance: TrainingPerformance::default(),
    }
}

#[test]
fn repeated_voice_training_inherits_latest_successful_pair_and_survives_parent_deletion() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let first = manager.create(parameters(), vec![clip(); 2]).unwrap();
    manager.run(&first.id).unwrap();
    let second = manager.create(parameters(), vec![clip(); 2]).unwrap();
    let prepared = store.prepared(&second.id).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&prepared.job_file).unwrap()).unwrap();
    assert_eq!(manifest["base"]["job_id"], first.id);
    assert_eq!(
        fs::read(prepared.work_dir.join("base/gpt.ckpt")).unwrap(),
        b"gpt model"
    );
    assert_eq!(
        fs::read(prepared.work_dir.join("base/sovits.pth")).unwrap(),
        b"sovits model"
    );
    manager.run(&second.id).unwrap();
    // Audition/save can update timestamps, but cannot make an older model the new base.
    manager.mark_auditioned(&first.id).unwrap();
    manager.save_version(&first.id).unwrap();
    let third = manager.create(parameters(), vec![clip(); 2]).unwrap();
    let third_prepared = store.prepared(&third.id).unwrap();
    let third_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&third_prepared.job_file).unwrap()).unwrap();
    assert_eq!(third_manifest["base"]["job_id"], second.id);
    manager.run(&third.id).unwrap();
    manager.delete(&second.id).unwrap();
    assert_eq!(
        fs::read(third_prepared.work_dir.join("base/gpt.ckpt")).unwrap(),
        b"gpt model"
    );
    assert!(manager.resolve_version(&third.id).is_ok());
    let mut other = parameters();
    other.voice_id = "different-voice".into();
    let different = manager.create(other, vec![clip(); 2]).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(store.prepared(&different.id).unwrap().job_file).unwrap())
            .unwrap();
    assert!(manifest["base"].is_null());
}

#[test]
fn damaged_latest_voice_pair_prevents_silent_restart_from_generic_weights() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let first = manager.create(parameters(), vec![clip(); 2]).unwrap();
    manager.run(&first.id).unwrap();
    fs::write(
        store
            .prepared(&first.id)
            .unwrap()
            .work_dir
            .join("artifacts/gpt.ckpt"),
        b"broken",
    )
    .unwrap();
    assert!(manager.create(parameters(), vec![clip(); 2]).is_err());
    assert_eq!(manager.snapshot().jobs.len(), 1);
    assert_eq!(fs::read_dir(temp.0.join("jobs")).unwrap().count(), 1);
}

#[test]
fn performance_is_persisted_in_catalog_and_job_manifest() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store,
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let mut configured = parameters();
    configured.performance = TrainingPerformance {
        batch_size: 4,
        data_workers: 0,
        cpu_threads: 8,
        gpu_index: 2,
        low_memory: false,
    };
    let job = manager.create(configured, vec![clip(); 2]).unwrap();
    let catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(temp.0.join("catalog.json")).unwrap()).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.0.join("jobs").join(job.id).join("job.json")).unwrap(),
    )
    .unwrap();
    let expected = serde_json::json!({
        "batch_size": 4, "data_workers": 0, "cpu_threads": 8,
        "gpu_index": 2, "low_memory": false
    });
    assert_eq!(catalog["jobs"][0]["performance"], expected);
    assert_eq!(manifest["performance"], expected);
}
struct FakeEngine {
    complete_pair: bool,
}
impl TrainingEngine for FakeEngine {
    fn run(
        &self,
        job: &PreparedTrainingJob,
        _: &AtomicBool,
        progress: &mut dyn FnMut(TrainingProgress),
    ) -> Result<ArtifactPair, TrainingError> {
        progress(TrainingProgress {
            state: TrainingState::Training,
            progress: 50,
        });
        fs::write(job.work_dir.join("artifacts/gpt.ckpt"), b"gpt model").unwrap();
        if self.complete_pair {
            fs::write(job.work_dir.join("artifacts/sovits.pth"), b"sovits model").unwrap();
            use sha2::{Digest, Sha256};
            let id: serde_json::Value =
                serde_json::from_slice(&fs::read(&job.job_file).unwrap()).unwrap();
            fs::write(job.work_dir.join("artifacts/manifest.json"),serde_json::to_vec(&serde_json::json!({
                "job_id":id["id"],"model_version":"v2","sha256":{"gpt":format!("{:x}",Sha256::digest(b"gpt model")),"sovits":format!("{:x}",Sha256::digest(b"sovits model"))}
            })).unwrap()).unwrap();
        }
        progress(TrainingProgress {
            state: TrainingState::Validating,
            progress: 95,
        });
        Ok(ArtifactPair::v2())
    }
}
#[test]
fn paired_success_needs_audition_and_survives_restart() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    assert_eq!(
        manager.run(&job.id).unwrap().state,
        TrainingState::Succeeded
    );
    assert_eq!(
        manager.activate(&job.id).unwrap_err(),
        TrainingError::NotAuditioned
    );
    manager.mark_auditioned(&job.id).unwrap();
    manager.activate(&job.id).unwrap();
    let persisted: serde_json::Value =
        serde_json::from_slice(&fs::read(temp.0.join("catalog.json")).unwrap()).unwrap();
    assert_eq!(persisted["jobs"][0]["saved"], true);
    let reopened = TrainingManager::open(
        store,
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    assert!(
        reopened
            .resolve_active_pair("default")
            .unwrap()
            .unwrap()
            .gpt
            .is_absolute()
    );
    assert_eq!(reopened.snapshot().active_versions["default"], job.id);
}

#[test]
fn saved_versions_survive_restart_and_switch_without_retraining() {
    let temp = Temp::new();
    let manager = TrainingManager::open(
        Arc::new(FileTrainingStore::open(&temp.0).unwrap()),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let mut ids = Vec::new();
    for (voice_id, name) in [
        ("default", "版本一"),
        ("default", "版本二"),
        ("another-voice", "另一音色"),
    ] {
        let mut input = parameters();
        input.voice_id = voice_id.into();
        input.name = name.into();
        let job = manager.create(input, vec![clip(); 2]).unwrap();
        manager.run(&job.id).unwrap();
        manager.mark_auditioned(&job.id).unwrap();
        assert!(!manager.snapshot().jobs.last().unwrap().saved);
        let saved = manager.save_version(&job.id).unwrap();
        assert!(saved.saved);
        assert_eq!(manager.save_version(&job.id).unwrap(), saved);
        ids.push(job.id);
    }
    assert!(manager.snapshot().active_versions.is_empty());
    drop(manager);
    let reopened = TrainingManager::open(
        Arc::new(FileTrainingStore::open(&temp.0).unwrap()),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    assert_eq!(reopened.snapshot().jobs.len(), 3);
    assert!(
        reopened
            .snapshot()
            .jobs
            .iter()
            .all(|job| job.saved && job.auditioned)
    );
    for index in [0, 1, 0] {
        reopened.activate(&ids[index]).unwrap();
        assert_eq!(reopened.snapshot().active_versions["default"], ids[index]);
        assert!(
            reopened
                .resolve_active_pair("default")
                .unwrap()
                .unwrap()
                .gpt
                .to_string_lossy()
                .contains(&ids[index])
        );
    }
    reopened.activate(&ids[2]).unwrap();
    assert_eq!(reopened.snapshot().active_versions["another-voice"], ids[2]);
    assert_eq!(reopened.snapshot().active_versions["default"], ids[0]);
    assert_eq!(
        FileTrainingStore::open(&temp.0)
            .unwrap()
            .load()
            .unwrap()
            .active_versions,
        reopened.snapshot().active_versions
    );
}

#[test]
fn saving_rejects_missing_unfinished_unauditioned_and_missing_artifacts() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    assert_eq!(
        manager.save_version("missing").unwrap_err(),
        TrainingError::NotFound
    );
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    assert_eq!(
        manager.save_version(&job.id).unwrap_err(),
        TrainingError::Busy
    );
    manager.run(&job.id).unwrap();
    assert_eq!(
        manager.save_version(&job.id).unwrap_err(),
        TrainingError::NotAuditioned
    );
    manager.mark_auditioned(&job.id).unwrap();
    fs::remove_file(manager.resolve_version(&job.id).unwrap().sovits).unwrap();
    assert!(manager.save_version(&job.id).is_err());
    assert!(!manager.snapshot().jobs[0].saved);
    assert!(!store.load().unwrap().jobs[0].saved);
    let mut other_parameters = parameters();
    other_parameters.voice_id = "other-voice".into();
    let cancelled = manager.create(other_parameters, vec![clip(); 2]).unwrap();
    manager.cancel(&cancelled.id).unwrap();
    assert!(matches!(
        manager.save_version(&cancelled.id),
        Err(TrainingError::Invalid(_))
    ));
}

#[test]
fn old_catalog_migrates_only_active_versions_to_saved() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    for active in [true, false] {
        let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
        manager.run(&job.id).unwrap();
        manager.mark_auditioned(&job.id).unwrap();
        if active {
            manager.activate(&job.id).unwrap();
        }
    }
    drop(manager);
    let catalog_path = temp.0.join("catalog.json");
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&fs::read(&catalog_path).unwrap()).unwrap();
    for job in legacy["jobs"].as_array_mut().unwrap() {
        job.as_object_mut().unwrap().remove("saved");
        job.as_object_mut().unwrap().remove("performance");
    }
    fs::write(&catalog_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let catalog = store.load().unwrap();
    assert!(catalog.jobs[0].saved);
    assert!(!catalog.jobs[1].saved);
    assert!(
        catalog
            .jobs
            .iter()
            .all(|job| { job.parameters.performance == TrainingPerformance::default() })
    );
    store.save(&catalog).unwrap();
    assert_eq!(store.load().unwrap(), catalog);
    // An explicit false in a current catalog is invalid for an active version.
    legacy["jobs"][0]["saved"] = serde_json::json!(false);
    fs::write(&catalog_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert!(store.load().is_err());
}

#[test]
fn catalog_rejects_saved_versions_without_successful_audition() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    let mut queued = store.load().unwrap();
    queued.jobs[0].saved = true;
    assert!(queued.validate().is_err());
    manager.run(&job.id).unwrap();
    let mut unauditioned = store.load().unwrap();
    unauditioned.jobs[0].saved = true;
    assert!(unauditioned.validate().is_err());
}
#[test]
fn partial_pair_cannot_be_successful() {
    let temp = Temp::new();
    let manager = TrainingManager::open(
        Arc::new(FileTrainingStore::open(&temp.0).unwrap()),
        Arc::new(FakeEngine {
            complete_pair: false,
        }),
    )
    .unwrap();
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    assert_eq!(manager.run(&job.id).unwrap().state, TrainingState::Failed);
    assert!(manager.resolve_version(&job.id).is_err());
}
#[test]
fn queued_restart_is_interrupted_and_bad_upload_leaves_no_job_directory() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let mut invalid = clip();
    invalid.wav[0] = 0;
    assert!(manager.create(parameters(), vec![invalid; 2]).is_err());
    assert_eq!(fs::read_dir(temp.0.join("jobs")).unwrap().count(), 0);
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    assert_eq!(
        manager.create(parameters(), vec![clip(); 2]).unwrap_err(),
        TrainingError::Busy
    );
    let reopened = TrainingManager::open(
        store,
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    assert_eq!(
        reopened.snapshot().jobs[0].state,
        TrainingState::Interrupted
    );
    assert!(reopened.run(&job.id).is_err());
}
#[cfg(unix)]
#[test]
fn symlink_artifacts_and_snapshot_are_rejected() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    manager.run(&job.id).unwrap();
    let pair = manager.resolve_version(&job.id).unwrap();
    fs::remove_file(&pair.gpt).unwrap();
    std::os::unix::fs::symlink(&pair.sovits, &pair.gpt).unwrap();
    assert!(manager.resolve_version(&job.id).is_err());
    fs::remove_file(temp.0.join("catalog.json")).unwrap();
    std::os::unix::fs::symlink(&pair.sovits, temp.0.join("catalog.json")).unwrap();
    assert!(store.load().is_err());
}
#[cfg(unix)]
#[test]
fn subprocess_cancellation_keeps_busy_until_owned_child_tree_exits() {
    use std::time::{Duration, Instant};
    let temp = Temp::new();
    let runner = temp.0.join("runner.py");
    fs::write(&runner, "import subprocess,sys,time,json,os\nchild=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)'])\nopen('child.pid','w').write(str(child.pid))\nprint(json.dumps({'state':'training','progress':20}),flush=True)\ntime.sleep(60)\n").unwrap();
    let engine = ProcessTrainingEngine::new(ProcessTrainingConfig {
        python: "/usr/bin/python3".into(),
        runner,
        timeout: Duration::from_secs(10),
    })
    .unwrap();
    let store = Arc::new(FileTrainingStore::open(temp.0.join("store")).unwrap());
    let manager = Arc::new(TrainingManager::open(store.clone(), Arc::new(engine)).unwrap());
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    let id = job.id.clone();
    let running = manager.clone();
    let thread = std::thread::spawn(move || running.run(&id));
    let child_file = store.prepared(&job.id).unwrap().work_dir.join("child.pid");
    let start = Instant::now();
    while !child_file.exists() {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        manager.cancel(&job.id).unwrap().state,
        TrainingState::Cancelling
    );
    // Cleanup may finish between cancel() and create(). Admission is valid only
    // after the old job is terminal and its owned child is no longer running.
    match manager.create(parameters(), vec![clip(); 2]) {
        Err(TrainingError::Busy) => {}
        Ok(next) => {
            assert_eq!(manager.snapshot().jobs[0].state, TrainingState::Cancelled);
            let pid = fs::read_to_string(&child_file).unwrap();
            if let Ok(stat) = fs::read_to_string(format!("/proc/{}/stat", pid.trim())) {
                assert_eq!(stat.split_whitespace().nth(2), Some("Z"));
            }
            manager.cancel(&next.id).unwrap();
        }
        Err(error) => panic!("unexpected admission error: {error}"),
    }
    assert_eq!(
        thread.join().unwrap().unwrap().state,
        TrainingState::Cancelled
    );
    assert!(!manager.snapshot().busy);
    let child_pid = fs::read_to_string(child_file).unwrap();
    if let Ok(stat) = fs::read_to_string(format!("/proc/{}/stat", child_pid.trim())) {
        assert_eq!(
            stat.split_whitespace().nth(2),
            Some("Z"),
            "owned child remained running"
        );
    }
}
#[cfg(unix)]
#[test]
fn subprocess_requires_success_event_and_honors_timeout() {
    use std::time::{Duration, Instant};
    let temp = Temp::new();
    let runner = temp.0.join("runner.py");
    fs::write(&runner, "import time\ntime.sleep(60)\n").unwrap();
    let engine = ProcessTrainingEngine::new(ProcessTrainingConfig {
        python: "/usr/bin/python3".into(),
        runner: runner.clone(),
        timeout: Duration::from_millis(100),
    })
    .unwrap();
    let prepared = PreparedTrainingJob {
        job_file: temp.0.join("job.json"),
        work_dir: temp.0.clone(),
    };
    fs::write(&prepared.job_file, "{}").unwrap();
    let start = Instant::now();
    assert!(
        engine
            .run(&prepared, &AtomicBool::new(false), &mut |_| {})
            .is_err()
    );
    assert!(start.elapsed() < Duration::from_secs(5));
    fs::write(&runner, "print('not a success event')\n").unwrap();
    assert!(
        engine
            .run(&prepared, &AtomicBool::new(false), &mut |_| {})
            .is_err()
    );
}

struct FailingCommitStore {
    inner: FileTrainingStore,
}
impl TrainingStore for FailingCommitStore {
    fn load(&self) -> Result<TrainingCatalog, TrainingError> {
        self.inner.load()
    }
    fn save(&self, _: &TrainingCatalog) -> Result<(), TrainingError> {
        Err(TrainingError::Store("模拟提交失败".into()))
    }
    fn new_job_id(&self) -> String {
        self.inner.new_job_id()
    }
    fn prepare(
        &self,
        job: &TrainingJob,
        clips: &[TrainingClip],
        base: Option<&TrainingJob>,
    ) -> Result<(), TrainingError> {
        self.inner.prepare(job, clips, base)
    }
    fn remove_unpublished(&self, id: &str) -> Result<(), TrainingError> {
        self.inner.remove_unpublished(id)
    }
    fn remove_published(&self, id: &str) -> Result<(), TrainingError> {
        self.inner.remove_published(id)
    }
    fn prepared(&self, id: &str) -> Result<PreparedTrainingJob, TrainingError> {
        self.inner.prepared(id)
    }
    fn resolve_pair(
        &self,
        id: &str,
        pair: &ArtifactPair,
    ) -> Result<meowlive_application::ports::training::ResolvedArtifactPair, TrainingError> {
        self.inner.resolve_pair(id, pair)
    }
}
#[test]
fn failed_catalog_commit_rolls_back_uploaded_dataset() {
    let temp = Temp::new();
    let manager = TrainingManager::open(
        Arc::new(FailingCommitStore {
            inner: FileTrainingStore::open(&temp.0).unwrap(),
        }),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    assert!(manager.create(parameters(), vec![clip(); 2]).is_err());
    assert!(manager.snapshot().jobs.is_empty());
    assert_eq!(fs::read_dir(temp.0.join("jobs")).unwrap().count(), 0);
}
#[test]
fn malformed_or_oversized_persistent_catalog_is_not_silently_reset() {
    let temp = Temp::new();
    let store = FileTrainingStore::open(&temp.0).unwrap();
    fs::write(
        temp.0.join("catalog.json"),
        r#"{"schema":2,"jobs":[],"active_versions":{}}"#,
    )
    .unwrap();
    assert!(store.load().is_err());
    fs::write(temp.0.join("catalog.json"), vec![b' '; 512 * 1024 + 1]).unwrap();
    assert!(store.load().is_err());
}
#[test]
fn clip_checks_reject_silent_audio_and_more_than_thirty_two_clips() {
    let temp = Temp::new();
    let manager = TrainingManager::open(
        Arc::new(FileTrainingStore::open(&temp.0).unwrap()),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let mut silent = clip();
    silent.wav[44..].fill(0);
    assert!(manager.create(parameters(), vec![silent; 2]).is_err());
    assert!(manager.create(parameters(), vec![clip(); 33]).is_err());
    let mut untranslated = clip();
    untranslated.language = "auto".into();
    assert!(manager.create(parameters(), vec![untranslated; 2]).is_err());
    assert_eq!(fs::read_dir(temp.0.join("jobs")).unwrap().count(), 0);
}
#[cfg(unix)]
#[test]
fn stderr_is_drained_into_a_bounded_private_log() {
    use std::time::Duration;
    let temp = Temp::new();
    let runner = temp.0.join("runner.py");
    fs::write(&runner, "import sys,json\nsys.stderr.write('x'*200000)\nprint(json.dumps({'state':'succeeded','progress':100}),flush=True)\n").unwrap();
    let engine = ProcessTrainingEngine::new(ProcessTrainingConfig {
        python: "/usr/bin/python3".into(),
        runner,
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let prepared = PreparedTrainingJob {
        job_file: temp.0.join("job.json"),
        work_dir: temp.0.clone(),
    };
    fs::write(&prepared.job_file, "{}").unwrap();
    assert!(
        engine
            .run(&prepared, &AtomicBool::new(false), &mut |_| {})
            .is_ok()
    );
    assert_eq!(
        fs::metadata(temp.0.join("engine.log")).unwrap().len(),
        64 * 1024
    );
}

#[test]
fn changed_weight_bytes_invalidate_audition_and_activation() {
    let temp = Temp::new();
    let manager = TrainingManager::open(
        Arc::new(FileTrainingStore::open(&temp.0).unwrap()),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    manager.run(&job.id).unwrap();
    manager.mark_auditioned(&job.id).unwrap();
    let pair = manager.resolve_version(&job.id).unwrap();
    fs::write(pair.gpt, b"modified model").unwrap();
    assert!(manager.resolve_version(&job.id).is_err());
    assert!(manager.activate(&job.id).is_err());
}

#[test]
fn same_size_weights_with_preserved_mtime_cannot_reuse_verified_hash() {
    for replace_file in [false, true] {
        let temp = Temp::new();
        let manager = TrainingManager::open(
            Arc::new(FileTrainingStore::open(&temp.0).unwrap()),
            Arc::new(FakeEngine {
                complete_pair: true,
            }),
        )
        .unwrap();
        let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
        manager.run(&job.id).unwrap();
        manager.mark_auditioned(&job.id).unwrap();
        let pair = manager.resolve_version(&job.id).unwrap();
        let original = fs::metadata(&pair.gpt).unwrap();
        let destination = if replace_file {
            pair.gpt.with_extension("replacement")
        } else {
            pair.gpt.clone()
        };
        fs::write(&destination, b"bad model").unwrap();
        fs::File::options()
            .write(true)
            .open(&destination)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(original.modified().unwrap()))
            .unwrap();
        if replace_file {
            fs::remove_file(&pair.gpt).unwrap();
            fs::rename(destination, &pair.gpt).unwrap();
        }
        let changed = fs::metadata(&pair.gpt).unwrap();
        assert_eq!(changed.len(), original.len());
        assert_eq!(changed.modified().unwrap(), original.modified().unwrap());
        assert!(manager.resolve_version(&job.id).is_err());
        assert!(manager.activate(&job.id).is_err());
    }
}

struct FailStartStore {
    inner: FileTrainingStore,
    saves: std::sync::atomic::AtomicUsize,
    keep_failing: AtomicBool,
}
impl TrainingStore for FailStartStore {
    fn load(&self) -> Result<TrainingCatalog, TrainingError> {
        self.inner.load()
    }
    fn save(&self, catalog: &TrainingCatalog) -> Result<(), TrainingError> {
        let call = self.saves.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if call == 1 || (call > 1 && self.keep_failing.load(std::sync::atomic::Ordering::SeqCst)) {
            return Err(TrainingError::Store("模拟启动状态写入失败".into()));
        }
        self.inner.save(catalog)
    }
    fn new_job_id(&self) -> String {
        self.inner.new_job_id()
    }
    fn prepare(
        &self,
        job: &TrainingJob,
        clips: &[TrainingClip],
        base: Option<&TrainingJob>,
    ) -> Result<(), TrainingError> {
        self.inner.prepare(job, clips, base)
    }
    fn remove_unpublished(&self, id: &str) -> Result<(), TrainingError> {
        self.inner.remove_unpublished(id)
    }
    fn remove_published(&self, id: &str) -> Result<(), TrainingError> {
        self.inner.remove_published(id)
    }
    fn prepared(&self, id: &str) -> Result<PreparedTrainingJob, TrainingError> {
        self.inner.prepared(id)
    }
    fn resolve_pair(
        &self,
        id: &str,
        pair: &ArtifactPair,
    ) -> Result<meowlive_application::ports::training::ResolvedArtifactPair, TrainingError> {
        self.inner.resolve_pair(id, pair)
    }
}
#[test]
fn failed_start_commit_releases_slot_without_starting_engine() {
    for persistent_failure in [false, true] {
        let temp = Temp::new();
        let store = Arc::new(FailStartStore {
            inner: FileTrainingStore::open(&temp.0).unwrap(),
            saves: std::sync::atomic::AtomicUsize::new(0),
            keep_failing: AtomicBool::new(persistent_failure),
        });
        let manager = TrainingManager::open(
            store.clone(),
            Arc::new(FakeEngine {
                complete_pair: true,
            }),
        )
        .unwrap();
        let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
        assert!(manager.run(&job.id).is_err());
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.jobs[0].state, TrainingState::Failed);
        assert!(!snapshot.busy);
        assert!(snapshot.jobs[0].message.contains("启动失败"));
        assert!(snapshot.jobs[0].artifacts.is_none());
        assert!(
            !store
                .prepared(&job.id)
                .unwrap()
                .work_dir
                .join("artifacts/gpt.ckpt")
                .exists()
        );
        assert!(
            manager.run(&job.id).is_err(),
            "failed jobs cannot be retried in place"
        );
        store
            .keep_failing
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let reopened = TrainingManager::open(
            store.clone(),
            Arc::new(FakeEngine {
                complete_pair: true,
            }),
        )
        .unwrap();
        assert_eq!(
            reopened.snapshot().jobs[0].state,
            if persistent_failure {
                TrainingState::Interrupted
            } else {
                TrainingState::Failed
            }
        );
        assert!(!reopened.snapshot().busy);
        assert!(manager.create(parameters(), vec![clip(); 2]).is_ok());
    }
}

#[test]
fn delete_active_saved_version_persists_and_preserves_other_versions() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let engine = Arc::new(FakeEngine {
        complete_pair: true,
    });
    let manager = TrainingManager::open(store.clone(), engine.clone()).unwrap();
    let mut ids = Vec::new();
    for _ in 0..2 {
        let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
        manager.run(&job.id).unwrap();
        manager.mark_auditioned(&job.id).unwrap();
        manager.save_version(&job.id).unwrap();
        ids.push(job.id);
    }
    manager.activate(&ids[0]).unwrap();
    manager.delete(&ids[0]).unwrap();
    assert_eq!(manager.snapshot().jobs.len(), 1);
    assert_eq!(manager.snapshot().jobs[0].id, ids[1]);
    assert!(manager.snapshot().active_versions.is_empty());
    assert!(!temp.0.join("jobs").join(&ids[0]).exists());
    assert!(manager.resolve_version(&ids[1]).is_ok());
    let reopened = TrainingManager::open(store, engine).unwrap();
    assert_eq!(reopened.snapshot().jobs, manager.snapshot().jobs);
    assert!(reopened.resolve_active_pair("default").unwrap().is_none());
    reopened.delete(&ids[0]).unwrap();
}

#[test]
fn delete_rejects_queued_training_and_allows_terminal_failed_jobs() {
    let temp = Temp::new();
    let manager = TrainingManager::open(
        Arc::new(FileTrainingStore::open(&temp.0).unwrap()),
        Arc::new(FakeEngine {
            complete_pair: false,
        }),
    )
    .unwrap();
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    assert_eq!(manager.delete(&job.id).unwrap_err(), TrainingError::Busy);
    assert!(temp.0.join("jobs").join(&job.id).exists());
    manager.run(&job.id).unwrap();
    let queued = manager.create(parameters(), vec![clip(); 2]).unwrap();
    assert_eq!(manager.delete(&job.id).unwrap_err(), TrainingError::Busy);
    manager.cancel(&queued.id).unwrap();
    manager.delete(&job.id).unwrap();
    manager.delete(&queued.id).unwrap();
    assert!(manager.snapshot().jobs.is_empty());
    assert_eq!(fs::read_dir(temp.0.join("jobs")).unwrap().count(), 0);
}

#[test]
fn delete_failed_catalog_commit_preserves_active_version_and_artifacts() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let engine = Arc::new(FakeEngine {
        complete_pair: true,
    });
    let manager = TrainingManager::open(store.clone(), engine.clone()).unwrap();
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    manager.run(&job.id).unwrap();
    manager.mark_auditioned(&job.id).unwrap();
    manager.activate(&job.id).unwrap();
    let before = store.load().unwrap();
    let failing = TrainingManager::open(
        Arc::new(FailingCommitStore {
            inner: FileTrainingStore::open(&temp.0).unwrap(),
        }),
        engine,
    )
    .unwrap();
    assert!(failing.delete(&job.id).is_err());
    assert_eq!(failing.snapshot().jobs, before.jobs);
    assert_eq!(store.load().unwrap(), before);
    assert!(failing.resolve_active_pair("default").unwrap().is_some());
}

#[test]
fn delete_cleanup_failure_is_explicit_and_retryable_after_restart() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let engine = Arc::new(FakeEngine {
        complete_pair: true,
    });
    let manager = TrainingManager::open(store.clone(), engine.clone()).unwrap();
    let job = manager.create(parameters(), vec![clip(); 2]).unwrap();
    manager.cancel(&job.id).unwrap();
    let directory = temp.0.join("jobs").join(&job.id);
    let held = temp.0.join("held");
    fs::rename(&directory, &held).unwrap();
    fs::write(&directory, b"cannot remove a non-directory").unwrap();
    let error = manager.delete(&job.id).unwrap_err().to_string();
    assert!(error.contains("记录已删除"));
    assert!(error.contains("重试"));
    assert!(manager.snapshot().jobs.is_empty());
    assert!(store.load().unwrap().jobs.is_empty());
    fs::remove_file(&directory).unwrap();
    fs::rename(&held, &directory).unwrap();
    let reopened = TrainingManager::open(store, engine).unwrap();
    reopened.delete(&job.id).unwrap();
    assert!(!directory.exists());
}

#[cfg(unix)]
#[test]
fn delete_rejects_traversal_and_job_symlinks_and_never_follows_nested_links() {
    let temp = Temp::new();
    let outside = Temp::new();
    fs::write(outside.0.join("keep"), b"keep").unwrap();
    let store = FileTrainingStore::open(&temp.0).unwrap();
    for id in ["..", "../outside", "/tmp", "", "jobs"] {
        assert!(store.remove_published(id).is_err());
    }
    let id = "00000000-0000-4000-8000-000000000001";
    let job = temp.0.join("jobs").join(id);
    std::os::unix::fs::symlink(&outside.0, &job).unwrap();
    assert!(store.remove_published(id).is_err());
    assert!(outside.0.join("keep").exists());
    fs::remove_file(&job).unwrap();
    fs::create_dir(&job).unwrap();
    std::os::unix::fs::symlink(&outside.0, job.join("nested")).unwrap();
    store.remove_published(id).unwrap();
    store.remove_published(id).unwrap();
    assert!(!job.exists());
    assert_eq!(fs::read(outside.0.join("keep")).unwrap(), b"keep");
}

#[test]
fn audio_only_dataset_is_accepted_and_records_automatic_transcription() {
    let temp = Temp::new();
    let store = Arc::new(FileTrainingStore::open(&temp.0).unwrap());
    let manager = TrainingManager::open(
        store.clone(),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let mut audio_only = clip();
    audio_only.text.clear();
    let job = manager.create(parameters(), vec![audio_only; 2]).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(store.prepared(&job.id).unwrap().job_file).unwrap())
            .unwrap();
    assert_eq!(manifest["text_mode"], "audio_only");
    assert_eq!(manifest["clips"][0]["text"], "");
}

#[test]
fn automatic_text_keeps_dataset_validation_and_rejects_partial_manual_text() {
    let temp = Temp::new();
    let manager = TrainingManager::open(
        Arc::new(FileTrainingStore::open(&temp.0).unwrap()),
        Arc::new(FakeEngine {
            complete_pair: true,
        }),
    )
    .unwrap();
    let mut audio_only = clip();
    audio_only.text.clear();
    assert!(
        manager
            .create(parameters(), vec![audio_only.clone(), clip()])
            .is_err()
    );
    for bad in [" ", "bad|text", "bad\ntext", "bad\u{85}text"] {
        let mut invalid = audio_only.clone();
        invalid.text = bad.into();
        assert!(manager.create(parameters(), vec![invalid; 2]).is_err());
    }
    audio_only.wav[0] = 0;
    assert!(manager.create(parameters(), vec![audio_only; 2]).is_err());
    assert!(manager.snapshot().jobs.is_empty());
}

#[cfg(unix)]
fn transcription_engine(
    temp: &Temp,
    script: &str,
    timeout: std::time::Duration,
) -> ProcessTrainingEngine {
    let runner = temp.0.join("runner.py");
    fs::write(&runner, "").unwrap();
    fs::write(temp.0.join("transcribe-training.py"), script).unwrap();
    let root = temp.0.join("engine");
    fs::create_dir_all(root.join("GPT_SoVITS")).unwrap();
    fs::write(root.join("GPT_SoVITS/s2_train.py"), "").unwrap();
    let storage = temp.0.join("storage");
    fs::create_dir(&storage).unwrap();
    ProcessTrainingEngine::new(ProcessTrainingConfig {
        python: "/usr/bin/python3".into(),
        runner,
        timeout,
    })
    .unwrap()
    .with_engine_root(root)
    .unwrap()
    .with_transcription(storage, temp.0.join("model"))
    .unwrap()
}

#[cfg(unix)]
#[test]
fn transcription_process_uses_owned_audio_returns_text_and_cleans_temporary_files() {
    let temp = Temp::new();
    let engine = transcription_engine(
        &temp,
        "import argparse,json,pathlib\np=argparse.ArgumentParser()\np.add_argument('--audio');p.add_argument('--language');p.add_argument('--engine-root');p.add_argument('--model')\na=p.parse_args()\nassert pathlib.Path(a.audio).is_file()\nassert pathlib.Path(a.audio).parent.parent.name == 'storage'\nassert a.language == 'zh'\nassert pathlib.Path(a.model).name == 'model'\nprint(json.dumps({'text':'识别的文本。','language':'zh'}))\n",
        std::time::Duration::from_secs(5),
    );
    let mut input = clip();
    input.text.clear();
    assert_eq!(
        engine.transcribe(&input, &AtomicBool::new(false)).unwrap(),
        "识别的文本。"
    );
    assert_eq!(fs::read_dir(temp.0.join("storage")).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn transcription_rejects_invalid_output_and_timeout_without_leaking_files() {
    for script in [
        "print('{}')",
        "print('{\"text\":\"bad|text\",\"language\":\"zh\"}')",
        "print('x'*20000)",
        "import time;time.sleep(60)",
    ] {
        let temp = Temp::new();
        let engine = transcription_engine(&temp, script, std::time::Duration::from_millis(150));
        let mut input = clip();
        input.text.clear();
        let error = engine
            .transcribe(&input, &AtomicBool::new(false))
            .unwrap_err()
            .to_string();
        assert!(error.contains("文本"), "{error}");
        assert_eq!(fs::read_dir(temp.0.join("storage")).unwrap().count(), 0);
    }
}

#[cfg(unix)]
#[test]
fn automatic_transcription_failure_reaches_job_status_as_actionable_text() {
    let temp = Temp::new();
    let runner = temp.0.join("runner.py");
    fs::write(&runner, "import json,sys\nprint(json.dumps({'state':'failed','error':'transcription_failed','progress':5}),flush=True)\nsys.exit(1)\n").unwrap();
    let engine = ProcessTrainingEngine::new(ProcessTrainingConfig {
        python: "/usr/bin/python3".into(),
        runner,
        timeout: std::time::Duration::from_secs(5),
    })
    .unwrap();
    let manager = TrainingManager::open(
        Arc::new(FileTrainingStore::open(temp.0.join("store")).unwrap()),
        Arc::new(engine),
    )
    .unwrap();
    let mut input = clip();
    input.text.clear();
    let job = manager.create(parameters(), vec![input; 2]).unwrap();
    let finished = manager.run(&job.id).unwrap();
    assert_eq!(finished.state, TrainingState::Failed);
    assert!(
        finished.message.contains("自动提取文本"),
        "{}",
        finished.message
    );
    assert!(
        finished.message.contains("手动填写"),
        "{}",
        finished.message
    );
}

#[cfg(unix)]
#[test]
fn transcription_holds_admission_until_shutdown_reaps_the_owned_process_tree() {
    use std::time::{Duration, Instant};
    let temp = Temp::new();
    let engine = transcription_engine(
        &temp,
        "import subprocess,sys,pathlib,time\np=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)'])\n(pathlib.Path.cwd().parent.parent/'asr-child.pid').write_text(str(p.pid))\ntime.sleep(60)\n",
        Duration::from_secs(30),
    );
    let manager = Arc::new(
        TrainingManager::open(
            Arc::new(FileTrainingStore::open(temp.0.join("store")).unwrap()),
            Arc::new(engine),
        )
        .unwrap(),
    );
    let mut input = clip();
    input.text.clear();
    let active = manager.clone();
    let next = input.clone();
    let thread = std::thread::spawn(move || active.transcribe(input));
    let start = Instant::now();
    let pid_file = temp.0.join("asr-child.pid");
    while !pid_file.exists() {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(manager.snapshot().busy);
    assert_eq!(manager.transcribe(next).unwrap_err(), TrainingError::Busy);
    assert_eq!(
        manager.create(parameters(), vec![clip(); 2]).unwrap_err(),
        TrainingError::Busy
    );
    manager.shutdown();
    assert_eq!(
        thread.join().unwrap().unwrap_err(),
        TrainingError::Cancelled
    );
    assert!(!manager.snapshot().busy);
    assert_eq!(fs::read_dir(temp.0.join("storage")).unwrap().count(), 0);
    let pid = fs::read_to_string(pid_file).unwrap();
    if let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) {
        assert_eq!(stat.split_whitespace().nth(2), Some("Z"));
    }
}
