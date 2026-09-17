//! 单片 CPU 识别的有界子进程与任务私有临时音频；不把机器路径暴露到接口。
use super::process::{OwnedChild, ProcessTrainingEngine};
use meowlive_domain::training::{TrainingClip, TrainingError};
use std::{
    fs,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

pub(super) fn run(
    engine: &ProcessTrainingEngine,
    clip: &TrainingClip,
    cancelled: &AtomicBool,
) -> Result<String, TrainingError> {
    clip.validate()?;
    super::store::validate_wav(&clip.wav)?;
    if cancelled.load(Ordering::SeqCst) {
        return Err(TrainingError::Cancelled);
    }
    let root = engine.transcription_root.as_ref().ok_or_else(failure)?;
    let metadata = fs::symlink_metadata(root).map_err(|_| failure())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(failure());
    }
    let directory =
        TemporaryDirectory::new(root.join(format!(".transcribe-{}", uuid::Uuid::new_v4())))?;
    let result = execute(engine, clip, cancelled, &directory);
    directory.cleanup()?;
    result
}

fn execute(
    engine: &ProcessTrainingEngine,
    clip: &TrainingClip,
    cancelled: &AtomicBool,
    directory: &TemporaryDirectory,
) -> Result<String, TrainingError> {
    let audio = directory.0.join("audio.wav");
    fs::write(&audio, &clip.wav).map_err(|_| failure())?;
    let runner = engine
        .config
        .runner
        .parent()
        .ok_or_else(failure)?
        .join("transcribe-training.py");
    let mut command = Command::new(&engine.config.python);
    command
        .arg("-s")
        .arg(runner)
        .arg("--audio")
        .arg(audio)
        .arg("--language")
        .arg(&clip.language)
        .arg("--engine-root")
        .arg(engine.engine_root.as_ref().ok_or_else(failure)?)
        .current_dir(&directory.0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONUNBUFFERED", "1");
    if let Some(model) = &engine.asr_model {
        command.arg("--model").arg(model);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut owned = OwnedChild {
        child: command.spawn().map_err(|_| failure())?,
        cleaned: false,
    };
    let stdout = owned.child.stdout.take().ok_or_else(failure)?;
    let stderr = owned.child.stderr.take().ok_or_else(failure)?;
    let output = thread::spawn(move || bounded_output(stdout, 8192));
    let diagnostic = thread::spawn(move || bounded_output(stderr, 64 * 1024));
    let start = Instant::now();
    let outcome = loop {
        if cancelled.load(Ordering::SeqCst) {
            break Err(TrainingError::Cancelled);
        }
        if start.elapsed() >= engine.config.timeout.min(Duration::from_secs(300)) {
            break Err(TrainingError::Engine(
                "自动提取文本超时，请手动填写文本后重试".into(),
            ));
        }
        match owned.child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err(failure())
                };
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(_) => break Err(failure()),
        }
    };
    owned.cleanup();
    let bytes = output.join().map_err(|_| failure());
    let _ = diagnostic.join();
    outcome?;
    let bytes = bytes??;
    let result: Transcript = serde_json::from_slice(&bytes).map_err(|_| failure())?;
    if result.language != clip.language || result.text.is_empty() {
        return Err(failure());
    }
    let checked = TrainingClip {
        text: result.text,
        language: result.language,
        wav: clip.wav.clone(),
    };
    checked.validate().map_err(|_| failure())?;
    Ok(checked.text)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Transcript {
    text: String,
    language: String,
}

fn bounded_output(mut stream: impl Read, maximum: usize) -> Result<Vec<u8>, TrainingError> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut exceeded = false;
    loop {
        let count = stream.read(&mut chunk).map_err(|_| failure())?;
        if count == 0 {
            break;
        }
        let available = maximum.saturating_sub(bytes.len());
        bytes.extend_from_slice(&chunk[..count.min(available)]);
        exceeded |= count > available;
    }
    if exceeded { Err(failure()) } else { Ok(bytes) }
}

struct TemporaryDirectory(PathBuf);
impl TemporaryDirectory {
    fn new(path: PathBuf) -> Result<Self, TrainingError> {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(|_| failure())?;
        Ok(Self(path))
    }
    fn cleanup(&self) -> Result<(), TrainingError> {
        fs::remove_dir_all(&self.0).map_err(|_| failure())
    }
}
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn failure() -> TrainingError {
    TrainingError::Engine("自动提取文本失败，请检查本地识别模型或手动填写文本".into())
}
