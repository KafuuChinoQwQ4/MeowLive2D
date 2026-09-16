//! Linux 音色资源保存目录及语音引擎共享挂载路径配置。
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourcesConfig {
    pub directory: PathBuf,
    pub engine_directory: String,
}
impl Default for ResourcesConfig {
    fn default() -> Self {
        Self {
            directory: "../data/resources".into(),
            engine_directory: String::new(),
        }
    }
}
impl ResourcesConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.directory.as_os_str().is_empty() || self.directory.to_string_lossy().len() > 4096 {
            return Err("resources.directory 必须为有效资源目录".into());
        }
        if !self.engine_directory.is_empty()
            && (!self.engine_directory.starts_with('/')
                || self.engine_directory.len() > 4096
                || self.engine_directory.contains(['\0', '\\']))
        {
            return Err("resources.engine_directory 必须是引擎可访问的 Linux 绝对目录".into());
        }
        Ok(())
    }
    pub fn resolve(&mut self, config_file: &Path) -> Result<(), String> {
        if self.directory.is_relative() {
            self.directory = config_file
                .parent()
                .unwrap_or(Path::new("."))
                .join(&self.directory);
        }
        if self.directory.is_relative() {
            self.directory = std::env::current_dir()
                .map_err(|_| "无法解析资源目录")?
                .join(&self.directory);
        }
        Ok(())
    }
}
