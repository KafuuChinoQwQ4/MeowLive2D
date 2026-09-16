//! 桌面资源命令执行；模型目录选择留在本机，VTS 请求不占用音频循环。
use crate::presentation::MouthControl;
use crate::{
    assets,
    avatar::{VtsConfig, resources::VtsResources},
    config::ClientConfig,
};
use meowlive_protocol::resources::{
    DesktopResourceOperation as Operation, DesktopResourceResult as Output, VtsHotkey, VtsModel,
};

#[derive(Clone)]
pub(crate) struct ResourceContext {
    pub config: ClientConfig,
    pub mouth: Option<MouthControl>,
}
impl ResourceContext {
    pub async fn execute(&self, operation: Operation) -> Output {
        match self.perform(operation).await {
            Ok(result) => result,
            Err(message) => Output::Error {
                code: "resource_failed".into(),
                message: message.chars().take(1000).collect(),
            },
        }
    }
    async fn perform(&self, operation: Operation) -> Result<Output, String> {
        if let Operation::Obs { operation } = operation {
            return crate::obs::execute(&self.config.obs, operation)
                .await
                .map(|snapshot| Output::Obs { snapshot });
        }
        if matches!(operation, Operation::ImportModel) {
            return self.import().await;
        }
        let mut vts = VtsResources::connect(&self.config.vtube_studio)
            .await
            .map_err(|e| e.to_string())?;
        match operation {
            Operation::ListModels => Ok(Output::Models {
                models: vts
                    .available_models()
                    .await
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|m| VtsModel {
                        id: m.id,
                        name: m.name,
                    })
                    .collect(),
            }),
            Operation::LoadModel {
                model_id,
                mouth_parameter,
            } => {
                let mut config: VtsConfig = self.config.vtube_studio.clone();
                config.mouth_parameter = mouth_parameter.clone();
                config.validate()?;
                let sender = self
                    .mouth
                    .as_ref()
                    .ok_or("桌面口型驱动未运行，无法切换角色")?;
                vts.load_model(&model_id).await.map_err(|e| e.to_string())?;
                sender.select(mouth_parameter).await?;
                Ok(Output::ModelLoaded { model_id })
            }
            Operation::ListHotkeys { model_id } => Ok(Output::Hotkeys {
                hotkeys: vts
                    .hotkeys(Some(&model_id))
                    .await
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|h| VtsHotkey {
                        id: h.id,
                        name: h.name,
                    })
                    .collect(),
                model_id,
            }),
            Operation::TriggerHotkey {
                model_id,
                hotkey_id,
                fallback_hotkey_id,
            } => {
                let hotkeys = vts
                    .hotkeys(Some(&model_id))
                    .await
                    .map_err(|e| e.to_string())?;
                let selected = if hotkeys.iter().any(|h| h.id == hotkey_id) {
                    hotkey_id
                } else {
                    fallback_hotkey_id
                        .filter(|id| hotkeys.iter().any(|h| h.id == *id))
                        .ok_or("模型热键及替代热键缺失，已跳过动作")?
                };
                vts.preview_hotkey(&model_id, &selected)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Output::HotkeyTriggered {
                    hotkey_id: selected,
                })
            }
            Operation::ImportModel | Operation::Obs { .. } => unreachable!(),
        }
    }
    async fn import(&self) -> Result<Output, String> {
        let target = self
            .config
            .model_directory
            .clone()
            .ok_or("请先配置桌面 model_directory 为 VTS 的 Live2DModels 目录")?;
        let source = choose_model_directory().await?;
        tokio::task::spawn_blocking(move || {
            let package =
                assets::validate_model_package(&source, &assets::ModelImportLimits::default())
                    .map_err(|e| e.to_string())?;
            let name = package
                .package_root
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("模型目录名称无效")?
                .to_owned();
            let install = assets::install_model_package(&package, &target, &name)
                .map_err(|e| e.to_string())?;
            Ok(Output::ModelImported {
                model_name: name,
                model_file: install.model_file.to_string_lossy().into_owned(),
                files: package.file_count as u32,
                bytes: package.total_bytes as u32,
                restart_required: true,
            })
        })
        .await
        .map_err(|_| "模型导入任务失败")?
    }
}

#[cfg(not(windows))]
async fn choose_model_directory() -> Result<std::path::PathBuf, String> {
    Err("请选择 Windows 执行端导入模型；Linux 仅支持命令行校验".into())
}

#[cfg(windows)]
async fn choose_model_directory() -> Result<std::path::PathBuf, String> {
    // Static PowerShell program: user-controlled text is never interpolated into code.
    let output=tokio::process::Command::new("powershell.exe")
        .args(["-NoProfile","-STA","-Command",r#"[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); Add-Type -AssemblyName System.Windows.Forms; $picker = New-Object System.Windows.Forms.FolderBrowserDialog; $picker.Description = 'MeowLive2D: Select an exported Live2D model folder'; if ($picker.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::Write($picker.SelectedPath) }"#])
        .kill_on_drop(true).output().await.map_err(|_|"无法打开 Windows 模型目录选择器")?;
    if !output.status.success() {
        return Err("Windows 模型目录选择器执行失败".into());
    }
    let path = String::from_utf8(output.stdout).map_err(|_| "模型目录编码无效")?;
    if path.trim().is_empty() {
        return Err("已取消模型导入".into());
    }
    Ok(path.into())
}
