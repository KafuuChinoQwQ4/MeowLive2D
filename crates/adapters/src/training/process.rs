//! 无 shell 的训练子进程；有界 JSON 进度与进程组退出保证。
use meowlive_application::ports::training::{
    PreparedTrainingJob, TrainingEngine, TrainingProgress,
};
use meowlive_domain::training::{ArtifactPair, TrainingError, TrainingState};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

pub struct ProcessTrainingConfig {
    pub python: PathBuf,
    pub runner: PathBuf,
    pub timeout: Duration,
}
pub struct ProcessTrainingEngine {
    config: ProcessTrainingConfig,
    engine_root: Option<PathBuf>,
}
impl ProcessTrainingEngine {
    pub fn new(config: ProcessTrainingConfig) -> Result<Self, TrainingError> {
        if !config.python.is_absolute()
            || !config.runner.is_absolute()
            || !config.python.is_file()
            || !config.runner.is_file()
            || config.timeout.is_zero()
            || config.timeout > Duration::from_secs(24 * 60 * 60)
        {
            return Err(TrainingError::Engine(
                "训练 Python、运行脚本或超时配置无效".into(),
            ));
        }
        Ok(Self {
            config,
            engine_root: None,
        })
    }
}
impl ProcessTrainingEngine {
    pub fn with_engine_root(mut self, root: PathBuf) -> Result<Self, TrainingError> {
        if !root.is_absolute() || !root.join("GPT_SoVITS/s2_train.py").is_file() {
            return Err(TrainingError::Engine("GPT-SoVITS 安装路径无效".into()));
        }
        self.engine_root = Some(root);
        Ok(self)
    }
}
impl TrainingEngine for ProcessTrainingEngine {
    fn run(
        &self,
        job: &PreparedTrainingJob,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(TrainingProgress),
    ) -> Result<ArtifactPair, TrainingError> {
        if cancelled.load(Ordering::SeqCst) {
            return Err(TrainingError::Cancelled);
        }
        if !job.job_file.is_absolute() || !job.work_dir.is_absolute() {
            return Err(engine_error());
        }
        let log = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(job.work_dir.join("engine.log"))
            .map_err(|_| engine_error())?;
        let mut command = Command::new(&self.config.python);
        if let Some(root) = &self.engine_root {
            command.env("MEOWLIVE_GPT_SOVITS_ROOT", root);
        }
        command.env("PYTHONDONTWRITEBYTECODE", "1");
        command
            .arg(&self.config.runner)
            .arg("--job")
            .arg(&job.job_file)
            .current_dir(&job.work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PYTHONUNBUFFERED", "1");
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command.spawn().map_err(|_| engine_error())?;
        let mut owned = OwnedChild {
            child,
            cleaned: false,
        };
        let stdout = owned.child.stdout.take().ok_or_else(engine_error)?;
        let stderr = owned.child.stderr.take().ok_or_else(engine_error)?;
        let log_reader = thread::spawn(move || drain_log(stderr, log));
        let (sender, receiver) = mpsc::sync_channel(32);
        let reader = thread::spawn(move || read_progress(stdout, sender));
        let start = Instant::now();
        let mut success = false;
        let mut last_progress = None;
        let mut last_emit = Instant::now() - Duration::from_secs(1);
        let outcome = loop {
            if cancelled.load(Ordering::SeqCst) {
                break Err(TrainingError::Cancelled);
            }
            if start.elapsed() >= self.config.timeout {
                break Err(TrainingError::Engine("训练超时，已终止训练进程".into()));
            }
            // Bound consumption per iteration so a noisy child cannot postpone cancellation.
            for _ in 0..32 {
                let Ok(event) = receiver.try_recv() else {
                    break;
                };
                if event.state == "succeeded" {
                    success = true;
                }
                if let Some(state) = TrainingState::parse(&event.state) {
                    if matches!(
                        state,
                        TrainingState::Preparing
                            | TrainingState::Training
                            | TrainingState::Validating
                    ) {
                        last_progress = Some(TrainingProgress {
                            state,
                            progress: event.progress.min(99),
                        });
                    }
                }
            }
            if last_emit.elapsed() >= Duration::from_millis(250) {
                if let Some(event) = last_progress.take() {
                    progress(event);
                    last_emit = Instant::now();
                }
            }
            match owned.child.try_wait() {
                Ok(Some(status)) => {
                    break if status.success() {
                        Ok(())
                    } else {
                        Err(engine_error())
                    };
                }
                Ok(None) => {}
                Err(_) => break Err(engine_error()),
            }
            thread::sleep(Duration::from_millis(25));
        };
        // Kill remaining descendants even if the direct runner exited successfully.
        owned.cleanup();
        let _ = reader.join();
        let _ = log_reader.join();
        for event in receiver.try_iter() {
            if event.state == "succeeded" {
                success = true;
            }
        }
        outcome?;
        if !success {
            return Err(TrainingError::Engine("训练引擎未确认完成".into()));
        }
        progress(TrainingProgress {
            state: TrainingState::Validating,
            progress: 99,
        });
        Ok(ArtifactPair::v2())
    }
}
#[derive(serde::Deserialize)]
struct Event {
    state: String,
    #[serde(default)]
    progress: u8,
}
fn read_progress(mut stdout: impl Read, sender: mpsc::SyncSender<Event>) {
    let mut chunk = [0u8; 4096];
    let mut line = Vec::with_capacity(4096);
    let mut oversized = false;
    loop {
        let Ok(count) = stdout.read(&mut chunk) else {
            return;
        };
        if count == 0 {
            return;
        }
        for byte in &chunk[..count] {
            if *byte == b'\n' {
                if !oversized {
                    if let Ok(event) = serde_json::from_slice::<Event>(&line) {
                        let _ = sender.try_send(event);
                    }
                }
                line.clear();
                oversized = false;
            } else if line.len() < 4096 {
                line.push(*byte);
            } else {
                oversized = true;
            }
        }
    }
}
struct OwnedChild {
    child: Child,
    cleaned: bool,
}
impl OwnedChild {
    fn cleanup(&mut self) {
        if self.cleaned {
            return;
        }
        terminate_group(self.child.id(), false);
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let done = self.child.try_wait().ok().flatten().is_some();
            if done && !group_running(self.child.id()) {
                break;
            }
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        terminate_group(self.child.id(), true);
        let _ = self.child.kill();
        let _ = self.child.wait();
        // SIGKILL is asynchronous; keep the GPU slot until no live member remains.
        #[cfg(target_os = "linux")]
        while group_running(self.child.id()) {
            thread::sleep(Duration::from_millis(10));
        }
        self.cleaned = true;
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.cleanup();
    }
}
fn terminate_group(id: u32, force: bool) {
    #[cfg(unix)]
    {
        let _ = nix::sys::signal::killpg(
            nix::unistd::Pid::from_raw(id as i32),
            if force {
                nix::sys::signal::Signal::SIGKILL
            } else {
                nix::sys::signal::Signal::SIGTERM
            },
        );
    }
    #[cfg(windows)]
    {
        let _ = force;
        let _ = Command::new("taskkill.exe")
            .args(["/PID", &id.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
fn group_running(id: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(entries) = fs::read_dir("/proc") else {
            return true;
        };
        for entry in entries.flatten() {
            if !entry
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|byte| byte.is_ascii_digit())
            {
                continue;
            }
            let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
                continue;
            };
            let Some((_, tail)) = stat.rsplit_once(") ") else {
                continue;
            };
            let fields: Vec<_> = tail.split_whitespace().take(4).collect();
            if fields.len() >= 3
                && fields[2].parse::<u32>().ok() == Some(id)
                && !matches!(fields[0], "Z" | "X")
            {
                return true;
            }
        }
        false
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = id;
        false
    }
}
fn engine_error() -> TrainingError {
    TrainingError::Engine("训练进程启动或运行失败".into())
}

fn drain_log(mut stderr: impl Read, mut log: fs::File) {
    let mut chunk = [0u8; 4096];
    let mut remaining = 64 * 1024;
    while let Ok(count) = stderr.read(&mut chunk) {
        if count == 0 {
            break;
        }
        let stored = count.min(remaining);
        if stored != 0 {
            let _ = log.write_all(&chunk[..stored]);
            remaining -= stored;
        }
    }
    let _ = log.sync_all();
}
