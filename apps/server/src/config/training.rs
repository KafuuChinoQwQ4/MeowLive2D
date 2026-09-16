//! 显式启用独立训练；安装路径只读，所有任务写入单独资源根。
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TrainingConfig {
    pub enabled: bool,
    pub directory: PathBuf,
    pub python: PathBuf,
    pub engine_root: PathBuf,
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
            directory: "../data/training".into(),
            python: PathBuf::new(),
            engine_root: PathBuf::new(),
            timeout_seconds: 7200,
            managed_inference: false,
            default_gpt_weights: PathBuf::new(),
            default_sovits_weights: PathBuf::new(),
        }
    }
}
impl TrainingConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.directory.as_os_str().is_empty() || !(60..=86400).contains(&self.timeout_seconds) {
            return Err("训练目录不能为空，训练超时须为 60–86400 秒".into());
        }
        if self.enabled && (!self.python.is_absolute() || !self.engine_root.is_absolute()) {
            return Err("启用训练须设置 Python 和 GPT-SoVITS 安装的绝对路径".into());
        }
        if self.managed_inference
            && (!self.default_gpt_weights.is_absolute()
                || !self.default_sovits_weights.is_absolute())
        {
            return Err("受管推理须设置默认 GPT 和 SoVITS 成对权重的绝对路径".into());
        }
        Ok(())
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
