//! 显式启用独立训练；安装路径只读，所有任务写入单独资源根。
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TrainingConfig {
    pub enabled: bool,
    pub wsl_distribution: String,
    pub wsl_python: String,
    pub wsl_engine_root: String,
    pub wsl_runner: String,
    pub wsl_bridge: String,
    pub directory: PathBuf,
    pub python: PathBuf,
    pub engine_root: PathBuf,
    pub asr_model: PathBuf,
    pub timeout_seconds: u64,
    /// 仅在独立受管配置启动的推理实例上启用模型权重管理。
    pub managed_inference: bool,
    pub default_gpt_weights: PathBuf,
    pub default_sovits_weights: PathBuf,
}
impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            wsl_distribution: String::new(),
            wsl_python: "/opt/meowlive-voice/venv/bin/python".into(),
            wsl_engine_root: "/opt/meowlive-voice/engine".into(),
            wsl_runner: "/opt/meowlive-voice/scripts/train-gpt-sovits.py".into(),
            wsl_bridge: "/opt/meowlive-voice/scripts/wsl-training-bridge.py".into(),
            directory: "../data/training".into(),
            python: PathBuf::new(),
            engine_root: PathBuf::new(),
            asr_model: PathBuf::new(),
            timeout_seconds: 7200,
            managed_inference: false,
            default_gpt_weights: PathBuf::new(),
            default_sovits_weights: PathBuf::new(),
        }
    }
}
impl TrainingConfig {
    pub fn validate(&self) -> Result<(), String> {
        let wsl = self.wsl_config()?;
        #[cfg(windows)]
        if (self.enabled || self.managed_inference) && wsl.is_none() {
            return Err("Windows 训练和受管推理须先选择兼容的 WSL2 发行版".into());
        }
        if self.directory.as_os_str().is_empty() || !(60..=86400).contains(&self.timeout_seconds) {
            return Err("训练目录不能为空，训练超时须为 60–86400 秒".into());
        }
        if self.enabled
            && wsl.is_none()
            && (!self.python.is_absolute() || !self.engine_root.is_absolute())
        {
            return Err("启用训练须设置 Python 和 GPT-SoVITS 安装的绝对路径".into());
        }
        if !self.asr_model.as_os_str().is_empty() && !absolute_model(&self.asr_model, wsl.is_some())
        {
            return Err("自动识别模型须为绝对路径，留空使用引擎默认模型目录".into());
        }
        if self.managed_inference
            && (!absolute_model(&self.default_gpt_weights, wsl.is_some())
                || !absolute_model(&self.default_sovits_weights, wsl.is_some()))
        {
            return Err("受管推理须设置默认 GPT 和 SoVITS 成对权重的绝对路径".into());
        }
        Ok(())
    }
    pub fn wsl_config(
        &self,
    ) -> Result<Option<meowlive_adapters::training::WslTrainingConfig>, String> {
        if self.wsl_distribution.is_empty() {
            return Ok(None);
        }
        let config = meowlive_adapters::training::WslTrainingConfig {
            distribution: self.wsl_distribution.clone(),
            python: self.wsl_python.clone(),
            engine_root: self.wsl_engine_root.clone(),
            runner: self.wsl_runner.clone(),
            bridge: self.wsl_bridge.clone(),
        };
        config.validate().map_err(|e| e.to_string())?;
        Ok(Some(config))
    }
    pub fn resolve(&mut self, config: &Path) -> Result<(), String> {
        if self.directory.is_relative() {
            self.directory = config
                .parent()
                .unwrap_or(Path::new("."))
                .join(&self.directory);
        }
        if self.directory.is_relative() {
            self.directory = std::env::current_dir()
                .map_err(|_| "无法解析训练目录")?
                .join(&self.directory);
        }
        let mut normalized = PathBuf::new();
        for component in self.directory.components() {
            match component {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                other => normalized.push(other.as_os_str()),
            }
        }
        self.directory = normalized;
        Ok(())
    }
}

fn absolute_model(path: &Path, wsl: bool) -> bool {
    path.is_absolute() || (wsl && path.to_string_lossy().starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wsl_config_accepts_linux_models_and_rejects_incomplete_backend() {
        let mut config = TrainingConfig {
            enabled: true,
            managed_inference: true,
            wsl_distribution: "Ubuntu-24.04".into(),
            asr_model: "/opt/models/asr".into(),
            default_gpt_weights: "/opt/models/gpt.ckpt".into(),
            default_sovits_weights: "/opt/models/sovits.pth".into(),
            ..TrainingConfig::default()
        };
        assert!(config.validate().is_ok());
        config.wsl_bridge = "relative/bridge.py".into();
        assert!(config.validate().is_err());
        config.wsl_bridge = "/opt/bridge.py".into();
        config.wsl_distribution = "--exec".into();
        assert!(config.validate().is_err());
    }
}
