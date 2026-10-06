//! Native Linux voice environment detection.
use super::{Backend, Distro, EnvironmentRequest, EnvironmentSnapshot, Model};
use std::{
    fs,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const CONTROLLER: &str = "/opt/meowlive-voice/scripts/voice-backend.py";

pub struct EnvironmentManager {
    path: PathBuf,
    state: Mutex<EnvironmentSnapshot>,
}

fn catalog() -> Vec<Model> {
    let items: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../../scripts/voice-backend-models.json"
    ))
    .expect("bundled model catalog");
    items
        .into_iter()
        .map(|item| Model {
            id: item["id"].as_str().unwrap_or_default().into(),
            name: item["name"].as_str().unwrap_or_default().into(),
            capability: if item["compatibility"] != "ready" {
                "download_only"
            } else if item["purpose"] == "asr" {
                "transcription"
            } else {
                "training_inference"
            }
            .into(),
            downloaded: false,
            selected: false,
        })
        .collect()
}

fn native_environment_name() -> &'static str {
    "Linux"
}

fn snapshot(path: &PathBuf) -> EnvironmentSnapshot {
    let saved: serde_json::Value = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let selected: Vec<String> = saved["selectedModels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect();
    let mut models = catalog();
    for model in &mut models {
        model.selected = selected.contains(&model.id);
    }
    EnvironmentSnapshot {
        phase: "idle".into(),
        busy: false,
        message: "检测本机 Linux 语音环境".into(),
        logs: vec![],
        distros: vec![Distro {
            name: native_environment_name().into(),
            // The shared frontend contract uses 1/2 for environment versions;
            // native Linux has no WSL version, so report the baseline value.
            version: 1,
        }],
        selected_distro: Some(native_environment_name().into()),
        backend: Backend::default(),
        models,
        progress: 0,
        inference_running: false,
    }
}

impl EnvironmentManager {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path: path.clone(),
            state: Mutex::new(snapshot(&path)),
        }
    }

    pub fn snapshot(&self) -> EnvironmentSnapshot {
        self.state.lock().unwrap().clone()
    }

    pub fn action(
        self: &Arc<Self>,
        request: EnvironmentRequest,
    ) -> Result<EnvironmentSnapshot, String> {
        match request.action.as_str() {
            "cancel" => {
                let _ = run_controller(&["cancel"], Duration::from_secs(10));
                self.detect()
            }
            "detect" => self.detect(),
            "install_backend" => Err("Linux 安装脚本需要系统包管理权限，当前桌面端不自动提权；请在终端运行 scripts/voice-backend-install.sh".into()),
            "download_model" => {
                let id = request.model_id.ok_or("请选择要下载的模型")?;
                self.set_busy("downloading", format!("正在下载模型 {id}"));
                let result = run_controller(&["download", &id], Duration::from_secs(4 * 60 * 60));
                match result {
                    Ok(_) => self.detect(),
                    Err(error) => self.fail(error),
                }
            }
            "select_model" => self.select_model(request.model_id.ok_or("请选择要使用的模型")?),
            "start_inference" => {
                let state = self.detect()?;
                let model = state
                    .models
                    .iter()
                    .find(|model| model.id == "gpt-sovits-v2")
                    .ok_or("模型目录中没有 GPT-SoVITS v2")?;
                if !model.downloaded || !model.selected {
                    return Err("请先下载并选用 GPT-SoVITS v2".into());
                }
                self.set_busy("running", "正在启动本机语音引擎".into());
                match run_controller(&["start"], Duration::from_secs(240)) {
                    Ok(_) => self.detect(),
                    Err(error) => self.fail(error),
                }
            }
            "stop_inference" => {
                self.set_busy("running", "正在停止本机语音引擎".into());
                match run_controller(&["stop"], Duration::from_secs(60)) {
                    Ok(_) => self.detect(),
                    Err(error) => self.fail(error),
                }
            }
            _ => Err("未知环境操作".into()),
        }
    }

    pub fn shutdown(&self) -> Result<(), String> {
        if self.snapshot().inference_running {
            let mut command = Command::new("python3");
            command.args([CONTROLLER, "stop"]);
            let status = command
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|error| error.to_string())?;
            if !status.success() {
                return Err("无法确认本机受管推理进程已停止".into());
            }
        }
        Ok(())
    }

    fn detect(&self) -> Result<EnvironmentSnapshot, String> {
        let result = run_controller(&["probe"], Duration::from_secs(30));
        let output = match result {
            Ok(output) => output,
            Err(error) => {
                let mut state = self.state.lock().unwrap();
                state.backend.detail = if error.contains("退出码 2") {
                    "尚未安装本机语音后端；请先在终端运行 scripts/voice-backend-install.sh".into()
                } else {
                    error
                };
                state.backend.ready = false;
                state.inference_running = false;
                state.phase = "idle".into();
                state.busy = false;
                return Ok(state.clone());
            }
        };
        let value: serde_json::Value = serde_json::from_str(&output)
            .map_err(|error| format!("语音后端检测响应无效：{error}"))?;
        let mut state = self.state.lock().unwrap();
        state.backend =
            serde_json::from_value(value["backend"].clone()).map_err(|e| e.to_string())?;
        let incoming: Vec<Model> =
            serde_json::from_value(value["models"].clone()).map_err(|e| e.to_string())?;
        for model in &mut state.models {
            if let Some(found) = incoming.iter().find(|item| item.id == model.id) {
                model.downloaded = found.downloaded;
                model.selected &= found.downloaded;
            }
        }
        state.inference_running = value["inferenceRunning"].as_bool().unwrap_or(false);
        state.phase = if state.inference_running {
            "running"
        } else {
            "idle"
        }
        .into();
        state.busy = false;
        state.message = "本机 Linux 语音环境已更新".into();
        Ok(state.clone())
    }

    fn select_model(&self, id: String) -> Result<EnvironmentSnapshot, String> {
        let state = self.detect()?;
        let selected = state
            .models
            .iter()
            .find(|model| model.id == id)
            .ok_or("模型不存在")?;
        if !selected.downloaded || selected.capability == "download_only" {
            return Err("该模型尚未下载或仅支持下载，不能选用".into());
        }
        if state.inference_running {
            return Err("语音引擎运行中，不能切换模型".into());
        }
        let mut next = self.state.lock().unwrap();
        let capability = selected.capability.clone();
        for model in &mut next.models {
            if model.capability == capability {
                model.selected = model.id == id;
            }
        }
        let selected_models: Vec<&str> = next
            .models
            .iter()
            .filter(|model| model.selected)
            .map(|model| model.id.as_str())
            .collect();
        let saved = serde_json::json!({ "selectedModels": selected_models });
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("无法创建配置目录：{error}"))?;
        }
        fs::write(
            &self.path,
            serde_json::to_vec_pretty(&saved).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("无法保存模型选择：{error}"))?;
        next.message = format!("已选用 {}", id);
        Ok(next.clone())
    }

    fn set_busy(&self, phase: &str, message: String) {
        let mut state = self.state.lock().unwrap();
        state.phase = phase.into();
        state.busy = true;
        state.message = message;
    }

    fn fail(&self, error: String) -> Result<EnvironmentSnapshot, String> {
        let mut state = self.state.lock().unwrap();
        state.phase = "failed".into();
        state.busy = false;
        state.message = error.clone();
        Err(error)
    }
}

fn run_controller(arguments: &[&str], timeout: Duration) -> Result<String, String> {
    let mut child = Command::new("python3")
        .arg(CONTROLLER)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法运行 Linux 语音控制器：{error}"))?;
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            let mut stdout = String::new();
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stdout.take() {
                let _ = pipe.read_to_string(&mut stdout);
            }
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr);
            }
            if status.success() {
                return Ok(stdout);
            }
            let detail = stderr.trim();
            return Err(if detail.is_empty() {
                format!("Linux 语音控制器失败（退出码 {:?}）", status.code())
            } else {
                detail.to_owned()
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return Err("Linux 语音控制器执行超时".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_name_is_native_linux() {
        assert_eq!(native_environment_name(), "Linux");
    }

    #[test]
    fn model_catalog_matches_controller_contract() {
        assert!(catalog().iter().any(|model| model.id == "gpt-sovits-v2"));
    }

    #[test]
    fn native_snapshot_uses_shared_environment_version_contract() {
        let state = snapshot(&std::env::temp_dir().join("meowlive-linux-test-config.json"));
        assert_eq!(state.distros[0].version, 1);
    }
}
