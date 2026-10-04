//! WSL argv boundary. Data remains owned by the Windows store.
use meowlive_domain::training::TrainingError;
use std::{path::PathBuf, process::Command};

#[derive(Clone, Debug)]
pub struct WslTrainingConfig {
    pub distribution: String,
    pub python: String,
    pub engine_root: String,
    pub runner: String,
    pub bridge: String,
}
impl WslTrainingConfig {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.distribution.trim().is_empty()
            || self.distribution.starts_with('-')
            || self.distribution.chars().any(char::is_control)
            || [&self.python, &self.engine_root, &self.runner, &self.bridge]
                .iter()
                .any(|p| !p.starts_with('/') || p.contains(['\0', '\n', '\r']))
        {
            return Err(TrainingError::Engine("WSL 训练配置无效".into()));
        }
        Ok(())
    }
    pub(super) fn command(&self, transcription: bool, model: Option<&PathBuf>) -> Command {
        let mut command = Command::new("wsl.exe");
        command.args([
            "--distribution",
            &self.distribution,
            "--exec",
            &self.python,
            "-s",
            &self.bridge,
            "--runner",
        ]);
        let runner = if transcription {
            self.runner
                .rsplit_once('/')
                .map(|(p, _)| format!("{p}/transcribe-training.py"))
                .unwrap_or_default()
        } else {
            self.runner.clone()
        };
        command
            .arg(runner)
            .args(["--engine-root", &self.engine_root]);
        if let Some(model) = model {
            command.arg("--model").arg(model);
        }
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridge_command_preserves_distribution_and_paths_as_argv() {
        let config = WslTrainingConfig {
            distribution: "Ubuntu Personal".into(),
            python: "/opt/voice env/python".into(),
            engine_root: "/opt/engine".into(),
            runner: "/opt/scripts/train-gpt-sovits.py".into(),
            bridge: "/opt/scripts/wsl-training-bridge.py".into(),
        };
        config.validate().unwrap();
        let command = config.command(true, Some(&PathBuf::from("/opt/asr model")));
        let args: Vec<_> = command.get_args().map(|p| p.to_str().unwrap()).collect();
        assert_eq!(
            args,
            [
                "--distribution",
                "Ubuntu Personal",
                "--exec",
                "/opt/voice env/python",
                "-s",
                "/opt/scripts/wsl-training-bridge.py",
                "--runner",
                "/opt/scripts/transcribe-training.py",
                "--engine-root",
                "/opt/engine",
                "--model",
                "/opt/asr model"
            ]
        );
    }
}
