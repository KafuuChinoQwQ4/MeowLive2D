//! Platform-specific voice environment management.
pub use meowlive_protocol::desktop_environment::{
    Backend, Distro, EnvironmentRequest, EnvironmentSnapshot, Model,
};
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::EnvironmentManager;

#[cfg(not(target_os = "linux"))]
mod windows {
    //! Native, repository-independent WSL environment management.
    use super::{Backend, Distro, EnvironmentRequest, EnvironmentSnapshot, Model};
    use serde::{Deserialize, Serialize};
    use std::{
        fs,
        io::{Read, Write},
        path::PathBuf,
        process::{Command, Stdio},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };
    const ROOT: &str = "/opt/meowlive-voice";
    const CONTROLLER: &str = "/opt/meowlive-voice/scripts/voice-backend.py";
    #[derive(Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Saved {
        schema: u8,
        selected_distro: Option<String>,
        selected_models: Vec<String>,
        snapshot: Option<EnvironmentSnapshot>,
    }
    pub struct EnvironmentManager {
        operation: Mutex<String>,
        monitoring: AtomicBool,
        state: Mutex<EnvironmentSnapshot>,
        path: PathBuf,
        cancel: AtomicBool,
        cache_loaded: bool,
    }
    fn catalog() -> Vec<Model> {
        let items: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../../scripts/voice-backend-models.json"
        ))
        .expect("bundled catalog");
        items
            .iter()
            .map(|v| Model {
                id: v["id"].as_str().unwrap().into(),
                name: v["name"].as_str().unwrap().into(),
                capability: if v["compatibility"] != "ready" {
                    "download_only"
                } else if v["purpose"] == "asr" {
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
    fn decode(bytes: &[u8]) -> String {
        if bytes.starts_with(&[255, 254])
            || bytes.iter().skip(1).step_by(2).filter(|b| **b == 0).count() > bytes.len() / 8
        {
            let values: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|x| u16::from_le_bytes([x[0], x[1]]))
                .collect();
            String::from_utf16_lossy(&values)
                .trim_start_matches('\u{feff}')
                .into()
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        }
    }
    // wsl.exe writes UTF-16LE; splitting at a single 0x0a leaves the following
    // 0x00 on the next line and corrupts every subsequent line, including Chinese.
    fn read_command_output(
        mut reader: impl Read,
        sender: std::sync::mpsc::SyncSender<String>,
        limit: usize,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        let mut pending = Vec::new();
        let mut utf16 = None;
        let mut chunk = [0; 8192];
        let emit = |line: &[u8], utf16: bool| {
            let text = if utf16 {
                String::from_utf16_lossy(
                    &line
                        .chunks_exact(2)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                        .collect::<Vec<_>>(),
                )
            } else {
                String::from_utf8_lossy(line).into_owned()
            };
            let text = text
                .trim_start_matches('\u{feff}')
                .trim_end_matches(['\r', '\n']);
            if !text.trim().is_empty() {
                let _ = sender.try_send(text.chars().take(1000).collect());
            }
        };
        while let Ok(size) = reader.read(&mut chunk) {
            if size == 0 {
                break;
            }
            data.extend_from_slice(&chunk[..size.min(limit.saturating_sub(data.len()))]);
            pending.extend_from_slice(&chunk[..size]);
            if utf16.is_none() && pending.len() >= 2 {
                utf16 = Some(pending.starts_with(&[255, 254]) || pending[1] == 0);
            }
            let Some(wide) = utf16 else {
                continue;
            };
            let mut start = 0;
            let width = if wide { 2 } else { 1 };
            for offset in (0..pending.len() / width * width).step_by(width) {
                if pending[offset] == b'\n' && (!wide || pending[offset + 1] == 0) {
                    emit(&pending[start..offset + width], wide);
                    start = offset + width;
                }
            }
            pending.drain(..start);
        }
        if !pending.is_empty() {
            emit(&pending, utf16.unwrap_or(false));
        }
        data
    }
    fn parse_distros(bytes: &[u8]) -> Result<Vec<Distro>, String> {
        let output = decode(bytes);
        let mut rows = Vec::new();
        for line in output.lines() {
            let line = line.trim().trim_start_matches('*').trim();
            let Some((prefix, version)) = line.rsplit_once(char::is_whitespace) else {
                continue;
            };
            let Ok(version) = version.parse::<u8>() else {
                continue;
            };
            if ![1, 2].contains(&version) {
                continue;
            }
            let Some((name, _state)) = prefix.trim_end().rsplit_once(char::is_whitespace) else {
                continue;
            };
            rows.push(Distro {
                name: name.trim().into(),
                version,
            });
        }
        if rows.is_empty() {
            Err(format!(
                "WSL 发行版探测没有返回可识别的列表：{}",
                output.trim()
            ))
        } else {
            Ok(rows)
        }
    }
    fn reset_distribution(state: &mut EnvironmentSnapshot, distro: Option<String>) {
        if state.selected_distro != distro {
            state.backend = Backend::default();
            state.inference_running = false;
            for model in &mut state.models {
                model.downloaded = false;
                model.selected = false;
            }
        }
        state.selected_distro = distro;
    }
    fn existing_training_paths(original: &str) -> Result<(String, String), String> {
        let doc: toml_edit::DocumentMut = original.parse().map_err(|_| "训练配置不是有效 TOML")?;
        let training = doc.get("training").ok_or("配置缺少 training 段")?;
        let path = |name: &str| -> Result<String, String> {
            training
                .get(name)
                .and_then(|v| v.as_str())
                .filter(|s| s.starts_with('/') && !s.chars().any(char::is_control))
                .map(str::to_owned)
                .ok_or_else(|| format!("training.{name} 必须是 WSL 绝对路径"))
        };
        Ok((path("python")?, path("engine_root")?))
    }
    fn command(program: &str) -> Command {
        #[allow(unused_mut)]
        let mut cmd = Command::new(program);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd
    }
    impl EnvironmentManager {
        pub fn new(path: PathBuf) -> Self {
            let saved: Saved = fs::read(&path)
                .ok()
                .and_then(|data| serde_json::from_slice(&data).ok())
                .unwrap_or_default();
            let cached = saved.snapshot.filter(|snapshot| {
                saved.schema == 1
                    && snapshot.models.len() == catalog().len()
                    && snapshot
                        .models
                        .iter()
                        .zip(catalog())
                        .all(|(actual, expected)| actual.id == expected.id)
            });
            let mut state = cached.unwrap_or_else(|| EnvironmentSnapshot {
                phase: "idle".into(),
                busy: false,
                message: "检测 WSL2 语音环境".into(),
                logs: vec![],
                distros: vec![],
                selected_distro: saved.selected_distro.clone(),
                backend: Backend::default(),
                models: catalog(),
                progress: 0,
                inference_running: false,
            });
            let cache_loaded = saved.schema == 1 && !state.distros.is_empty();
            if !cache_loaded {
                for item in &mut state.models {
                    item.selected = saved.selected_models.contains(&item.id);
                }
            }
            // A previous process may have exited while inference was active. Runtime
            // activity and logs are never restored from the persistent probe cache.
            state.busy = false;
            state.logs.clear();
            state.inference_running = false;
            state.progress = if cache_loaded { 100 } else { 0 };
            if cache_loaded {
                state.phase = "idle".into();
                state.message = "已恢复上次环境检测结果，正在快速校验 WSL 状态".into();
            }
            Self {
                operation: Mutex::new(String::new()),
                monitoring: AtomicBool::new(false),
                path,
                cancel: AtomicBool::new(false),
                cache_loaded,
                state: Mutex::new(state),
            }
        }
        pub fn validate_cached_state(self: &Arc<Self>) {
            let manager = Arc::clone(self);
            std::thread::spawn(move || {
                let distro_matches = if manager.cache_loaded {
                    let mut command = command("wsl.exe");
                    command.args(["--list", "--verbose"]);
                    manager
                        .capture(command, Duration::from_secs(30))
                        .and_then(|bytes| parse_distros(&bytes))
                        .is_ok_and(|distros| {
                            let cached = manager.snapshot().distros;
                            cached.len() == distros.len()
                                && cached.iter().all(|item| {
                                    distros.iter().any(|actual| {
                                        actual.name == item.name && actual.version == item.version
                                    })
                                })
                        })
                } else {
                    false
                };
                if !distro_matches {
                    let selected = manager.snapshot().selected_distro;
                    let _ = manager.action(EnvironmentRequest {
                        action: "detect".into(),
                        distro: selected,
                        model_id: None,
                        config_path: None,
                    });
                }
            });
        }
        pub fn snapshot(&self) -> EnvironmentSnapshot {
            self.state.lock().unwrap().clone()
        }
        fn log(&self, line: String) {
            let mut s = self.state.lock().unwrap();
            s.logs.push(line);
            if s.logs.len() > 200 {
                s.logs.remove(0);
            }
        }
        fn save(&self) -> Result<(), String> {
            let state = self.snapshot();
            let saved = Saved {
                schema: 1,
                selected_distro: state.selected_distro.clone(),
                selected_models: state
                    .models
                    .iter()
                    .filter(|x| x.selected)
                    .map(|x| x.id.clone())
                    .collect(),
                snapshot: Some(EnvironmentSnapshot {
                    phase: "idle".into(),
                    busy: false,
                    message: "环境任务完成".into(),
                    logs: Vec::new(),
                    distros: state.distros,
                    selected_distro: state.selected_distro,
                    backend: state.backend,
                    models: state.models,
                    progress: 100,
                    inference_running: false,
                }),
            };
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let temporary = self.path.with_extension("json.part");
            fs::write(&temporary, serde_json::to_vec(&saved).unwrap())
                .map_err(|e| e.to_string())?;
            fs::rename(temporary, &self.path).map_err(|e| e.to_string())
        }
        pub fn action(
            self: &Arc<Self>,
            request: EnvironmentRequest,
        ) -> Result<EnvironmentSnapshot, String> {
            if request.action == "cancel" {
                if !self.snapshot().busy {
                    return Ok(self.snapshot());
                }
                if self.operation.lock().unwrap().as_str() == "install_wsl" {
                    return Err("Windows 安装器正在运行，请在系统安装窗口取消或等待完成".into());
                }
                self.cancel.store(true, Ordering::SeqCst);
                self.stop_remote("cancel")?;
                return Ok(self.snapshot());
            }
            {
                let mut s = self.state.lock().unwrap();
                if s.busy {
                    return Err("环境任务正在运行".into());
                }
                s.busy = true;
                s.progress = 0;
                s.phase = match request.action.as_str() {
                    "detect" => "detecting",
                    "download_model" => "downloading",
                    _ => "installing",
                }
                .into();
                s.message = "正在处理环境任务".into();
            }
            *self.operation.lock().unwrap() = request.action.clone();
            self.cancel.store(false, Ordering::SeqCst);
            let this = Arc::clone(self);
            std::thread::spawn(move || {
                let action = request.action.clone();
                let result = this.execute(request);
                if result.is_err()
                    && matches!(
                        action.as_str(),
                        "install_backend" | "download_model" | "start_inference"
                    )
                    || (result.is_err() && action == "detect")
                {
                    if let Err(error) = this.stop_remote("cancel") {
                        this.log(format!("停止后台任务失败：{error}"));
                    }
                }
                if result.is_ok() && this.snapshot().inference_running {
                    this.monitor();
                }
                let mut state = this.state.lock().unwrap();
                if result.is_err() && action == "detect" {
                    state.backend = Backend::default();
                    for model in &mut state.models {
                        model.downloaded = false;
                    }
                }
                state.busy = false;
                match result {
                    Ok(()) => {
                        if state.phase != "reboot_required" {
                            state.phase = if state.inference_running {
                                "running"
                            } else {
                                "idle"
                            }
                            .into();
                        }
                        state.progress = 100;
                        state.message = if state.phase == "reboot_required" {
                        "WSL 安装已完成。若系统要求请先重启；从开始菜单打开 Ubuntu-22.04，完成首次用户名与密码设置后，再回来检测。"
                    } else {
                        "环境任务完成"
                    }
                    .into();
                    }
                    Err(error) => {
                        state.phase = "failed".into();
                        state.message = error.clone();
                        state.logs.push(error);
                    }
                }
            });
            Ok(self.snapshot())
        }
        fn monitor(self: &Arc<Self>) {
            if self.monitoring.swap(true, Ordering::SeqCst) {
                return;
            }
            let this = Arc::clone(self);
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(3));
                    let state = this.snapshot();
                    if !state.inference_running {
                        break;
                    }
                    if state.busy {
                        continue;
                    }
                    let Some(distro) = state.selected_distro else {
                        break;
                    };
                    let mut cmd = this.wsl(&distro);
                    cmd.args(["python3", CONTROLLER, "status"]);
                    let running = this
                        .run(cmd, None, Duration::from_secs(15))
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                        .and_then(|value| value["inferenceRunning"].as_bool())
                        .unwrap_or(false);
                    if !running {
                        let mut state = this.state.lock().unwrap();
                        state.inference_running = false;
                        if !state.busy {
                            state.phase = "idle".into();
                            state.message = "推理服务已停止".into();
                        }
                        break;
                    }
                }
                this.monitoring.store(false, Ordering::SeqCst);
            });
        }
        fn run(
            &self,
            cmd: Command,
            input: Option<Vec<u8>>,
            timeout: Duration,
        ) -> Result<Vec<u8>, String> {
            let quiet = cmd
                .get_args()
                .last()
                .is_some_and(|arg| arg == "status" || arg == "read-config");
            self.run_command(cmd, input, timeout, quiet)
        }
        fn capture(&self, cmd: Command, timeout: Duration) -> Result<Vec<u8>, String> {
            self.run_command(cmd, None, timeout, true)
        }
        fn run_command(
            &self,
            mut cmd: Command,
            input: Option<Vec<u8>>,
            timeout: Duration,
            quiet: bool,
        ) -> Result<Vec<u8>, String> {
            let control = cmd
                .get_args()
                .last()
                .is_some_and(|arg| arg == "cancel" || arg == "stop");
            let mut child = cmd
                .stdin(if input.is_some() {
                    Stdio::piped()
                } else {
                    Stdio::null()
                })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| e.to_string())?;
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let (sender, receiver) = std::sync::mpsc::sync_channel::<String>(128);
            let out_sender = sender.clone();
            let out = std::thread::spawn(move || {
                read_command_output(stdout, out_sender, 2 * 1024 * 1024)
            });
            let err = std::thread::spawn(move || read_command_output(stderr, sender, 128 * 1024));
            if let Some(input) = input {
                if let Some(mut stdin) = child.stdin.take() {
                    std::thread::spawn(move || {
                        let _ = stdin.write_all(&input);
                    });
                }
            }
            let started = Instant::now();
            loop {
                for line in receiver.try_iter() {
                    if !quiet && !line.trim().is_empty() {
                        self.log(line);
                    }
                }
                if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                    let output = out.join().unwrap_or_default();
                    let errors = err.join().unwrap_or_default();
                    for line in receiver.try_iter() {
                        if !quiet {
                            self.log(line);
                        }
                    }
                    if !status.success() {
                        if quiet {
                            return Err("读取环境状态或配置失败，请检查所选发行版及文件路径".into());
                        }
                        return Err(format!(
                            "命令失败 ({status})：{} {}",
                            decode(&output).chars().take(4000).collect::<String>(),
                            decode(&errors).chars().take(4000).collect::<String>()
                        ));
                    }
                    return Ok(output);
                }
                if started.elapsed() > timeout || (!control && self.cancel.load(Ordering::SeqCst)) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("环境任务已取消或超时；可以重新检测后重试".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        fn wsl(&self, distro: &str) -> Command {
            let mut cmd = command("wsl.exe");
            cmd.args(["-d", distro, "-u", "root", "--"]);
            cmd
        }
        fn selected(&self) -> Result<String, String> {
            let s = self.snapshot();
            let name = s.selected_distro.ok_or("请先选择 WSL2 发行版")?;
            if !s.distros.iter().any(|d| d.name == name && d.version == 2) {
                return Err("仅支持检测到的 WSL2 发行版".into());
            }
            Ok(name)
        }
        fn stop_remote(&self, action: &str) -> Result<(), String> {
            let Some(distro) = self.snapshot().selected_distro else {
                return Ok(());
            };
            let mut cmd = self.wsl(&distro);
            cmd.args(["python3", "-c", "import pathlib,runpy,sys; p=pathlib.Path('/opt/meowlive-voice/scripts/voice-backend.py'); sys.argv=[str(p),sys.argv[1]]; runpy.run_path(str(p),run_name='__main__') if p.is_file() else None", action]);
            self.run(cmd, None, Duration::from_secs(15))?;
            Ok(())
        }
        pub fn shutdown(&self) -> Result<(), String> {
            if self.snapshot().busy {
                let operation = self.operation.lock().unwrap().clone();
                if operation == "install_wsl" {
                    return Err("Windows 安装器仍在运行，请等待完成或在系统窗口取消后再退出".into());
                }
                let remote_task = matches!(
                    operation.as_str(),
                    "install_backend" | "download_model" | "start_inference"
                ) || operation == "detect";
                self.cancel.store(true, Ordering::SeqCst);
                self.log("正在取消环境任务并等待后台进程退出".into());
                // Keep the app alive if cancellation cannot be confirmed. Killing wsl.exe
                // alone does not terminate the Linux process group.
                if remote_task {
                    self.stop_remote("cancel")?;
                }
                let started = Instant::now();
                while self.snapshot().busy {
                    if started.elapsed() > Duration::from_secs(30) {
                        return Err("后台任务尚未退出，已阻止关闭；请稍后重试".into());
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                // Cover startup/cancel races: a newly spawned Linux group may have
                // registered its PID after the first cancellation request.
                if remote_task {
                    self.stop_remote("cancel")?;
                }
            }
            if self.snapshot().backend.ready || self.snapshot().inference_running {
                self.stop_remote("stop")?;
            }
            self.state.lock().unwrap().inference_running = false;
            Ok(())
        }
        fn deploy(&self, distro: &str) -> Result<(), String> {
            let bundle = serde_json::json!({
            "voice-backend.py":include_str!("../../../../../scripts/voice-backend.py"),
            "voice-backend-models.json":include_str!("../../../../../scripts/voice-backend-models.json"),
            "voice-backend-install.sh":include_str!("../../../../../scripts/voice-backend-install.sh"),
            "train-gpt-sovits.py":include_str!("../../../../../scripts/train-gpt-sovits.py"),
            "transcribe-training.py":include_str!("../../../../../scripts/transcribe-training.py"),
            "wsl-training-bridge.py":include_str!("../../../../../scripts/wsl-training-bridge.py"),
            "engine_workspace.py":include_str!("../../../../../scripts/engine_workspace.py"),
            "training_transcription.py":include_str!("../../../../../scripts/training_transcription.py"),
            "model_runtime.py":include_str!("../../../../../scripts/model_runtime.py"),
            "start-managed-inference.py":include_str!("../../../../../scripts/start-managed-inference.py"),
            "extract-model-archive.py":include_str!("../../../../../scripts/extract-model-archive.py")});
            let mut cmd = self.wsl(distro);
            cmd.args(["python3","-c","import json,sys,pathlib,os; os.umask(0o022); p=pathlib.Path('/opt/meowlive-voice/scripts'); p.mkdir(parents=True,exist_ok=True); [(p/k).write_text(v) for k,v in json.load(sys.stdin).items()]"]);
            self.run(
                cmd,
                Some(serde_json::to_vec(&bundle).unwrap()),
                Duration::from_secs(30),
            )?;
            Ok(())
        }
        fn probe(&self, distro: &str) -> Result<(), String> {
            self.probe_config(distro, None)
        }
        fn probe_config(&self, distro: &str, config_path: Option<&str>) -> Result<(), String> {
            self.deploy(distro)?;
            let mut cmd = self.wsl(distro);
            cmd.args(["python3", CONTROLLER, "probe"]);
            let explicit = config_path.filter(|p| !p.trim().is_empty());
            if explicit
                .is_some_and(|path| !path.starts_with('/') || path.chars().any(char::is_control))
            {
                return Err("请填写配置文件在 WSL 中的绝对路径".into());
            }
            let mut read = self.wsl(distro);
            // Keep full configuration out of the UI log: it may contain credentials.
            read.args([
                "python3",
                "-c",
                r#"import pathlib,sys
root=pathlib.Path('/opt/meowlive-voice')
explicit=sys.argv[1]
paths=[pathlib.Path(explicit)] if explicit else []
if not explicit and not (root/'backend.json').exists() and not (root/'existing.json').exists():
    homes=[pathlib.Path('/root'), *pathlib.Path('/home').glob('*')]
    paths=[h/p/'config/server.local.toml' for h in homes for p in ['code/MeowLive2D', 'MeowLive2D']]
for p in paths:
    if explicit or p.is_file():
        assert p.stat().st_size <= 1048576, '配置文件过大'
        sys.stdout.buffer.write(p.read_bytes())
        break
"#,
                explicit.unwrap_or(""),
                "read-config",
            ]);
            let bytes = self.run(read, None, Duration::from_secs(15))?;
            if !bytes.is_empty() {
                let (python, engine) = existing_training_paths(&decode(&bytes))?;
                cmd.args(["--existing-python", &python, "--existing-engine", &engine]);
            }
            let output = self
            .capture(cmd, Duration::from_secs(90))
            .map_err(|error| {
                format!(
                    "{error}。新安装的 Ubuntu 请先从开始菜单启动并完成用户名和密码设置，再重新检测"
                )
            })?;
            let value: serde_json::Value =
                serde_json::from_slice(&output).map_err(|e| format!("后端检测响应无效：{e}"))?;
            let mut s = self.state.lock().unwrap();
            s.backend =
                serde_json::from_value(value["backend"].clone()).map_err(|e| e.to_string())?;
            let mut models: Vec<Model> =
                serde_json::from_value(value["models"].clone()).map_err(|e| e.to_string())?;
            for item in &mut models {
                item.selected =
                    s.models.iter().any(|x| x.id == item.id && x.selected) && item.downloaded;
            }
            s.models = models;
            s.inference_running = value["inferenceRunning"].as_bool().unwrap_or(false);
            let detail = s.backend.detail.clone();
            let model_ready = s
                .models
                .iter()
                .any(|m| m.id == "gpt-sovits-v2" && m.downloaded);
            let model_selected = s
                .models
                .iter()
                .any(|m| m.id == "gpt-sovits-v2" && m.selected);
            drop(s);
            self.log(format!("后端检测：{detail}"));
            self.log(format!(
                "GPT-SoVITS v2 模型：{}",
                if model_selected {
                    "已选用"
                } else if model_ready {
                    "已检测完整，等待选用"
                } else {
                    "文件不完整，请下载模型"
                }
            ));
            Ok(())
        }
        fn execute(&self, request: EnvironmentRequest) -> Result<(), String> {
            if !cfg!(windows) {
                return Err("WSL 环境管理仅在 Windows 桌面应用可用".into());
            }
            if request.action == "install_wsl" {
                self.log("请求管理员安装官方 Ubuntu-22.04 / WSL2；不会自动重启".into());
                let mut cmd = command("powershell.exe");
                cmd.args(["-NoProfile","-NonInteractive","-Command","$p=Start-Process -FilePath wsl.exe -ArgumentList '--install','--distribution','Ubuntu-22.04','--no-launch' -Verb RunAs -Wait -PassThru; if ($p.ExitCode -ne 0 -and $p.ExitCode -ne 3010) { exit $p.ExitCode }"]);
                self.run(cmd, None, Duration::from_secs(1800))?;
                self.state.lock().unwrap().phase = "reboot_required".into();
                return Ok(());
            }
            if request.action == "detect" {
                // Registration is structured and locale independent; an empty registration
                // list means no distro, whereas command errors with registrations are failures.
                let mut registry = command("powershell.exe");
                registry.args(["-NoProfile", "-NonInteractive", "-Command", r"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); $p='HKCU:\Software\Microsoft\Windows\CurrentVersion\Lxss'; if(Test-Path $p){$n=@(Get-ChildItem $p -ErrorAction Stop | ForEach-Object {(Get-ItemProperty $_.PSPath -ErrorAction Stop).DistributionName} | Where-Object {$_}); Write-Output $n.Count} else {Write-Output 0}"]);
                let registrations = self.capture(registry, Duration::from_secs(15))?;
                if decode(&registrations)
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "无法读取 WSL 注册信息")?
                    == 0
                {
                    let mut s = self.state.lock().unwrap();
                    s.distros.clear();
                    reset_distribution(&mut s, None);
                    s.backend = Backend {
                    detail: "尚未注册 WSL 发行版。安装后请从开始菜单启动 Ubuntu-22.04 并完成首次账户设置，再回来检测".into(),
                    ..Default::default()
                };
                    s.inference_running = false;
                    for item in &mut s.models {
                        item.downloaded = false;
                    }
                    return Ok(());
                }
                let mut cmd = command("wsl.exe");
                cmd.args(["--list", "--verbose"]);
                let output = self.capture(cmd, Duration::from_secs(30))?;
                let distros = parse_distros(&output)?;
                self.log(format!(
                    "发现 WSL 环境：{}",
                    distros
                        .iter()
                        .map(|d| format!("{}（WSL{}）", d.name, d.version))
                        .collect::<Vec<_>>()
                        .join("、")
                ));
                let mut s = self.state.lock().unwrap();
                if s.inference_running
                    && request.distro.is_some()
                    && request.distro != s.selected_distro
                {
                    return Err("请先停止推理再切换发行版".into());
                }
                s.distros = distros;
                let desired = request.distro.clone().or_else(|| s.selected_distro.clone());
                let desired = desired
                    .filter(|name| s.distros.iter().any(|d| &d.name == name && d.version == 2))
                    .or_else(|| {
                        s.distros
                            .iter()
                            .find(|d| d.version == 2)
                            .map(|d| d.name.clone())
                    });
                reset_distribution(&mut s, desired);
                drop(s);
                let distro = self.selected()?;
                self.probe_config(&distro, request.config_path.as_deref())?;
                return self.save();
            }
            if let Some(distro) = request.distro {
                let mut s = self.state.lock().unwrap();
                if s.inference_running && s.selected_distro.as_ref() != Some(&distro) {
                    return Err("请先停止推理再切换发行版".into());
                }
                if !s.distros.iter().any(|d| d.name == distro && d.version == 2) {
                    return Err("请选择检测到的 WSL2 发行版".into());
                }
                reset_distribution(&mut s, Some(distro));
                drop(s);
                self.selected()?;
            }
            let distro = self.selected()?;
            match request.action.as_str() {
                "install_backend" => {
                    self.probe(&distro)?;
                    if self.snapshot().inference_running {
                        return Err("请先停止推理".into());
                    }
                    if self.snapshot().backend.ready
                        && self.snapshot().backend.engine_root != format!("{ROOT}/engine")
                    {
                        return Err("已识别现有训练环境，请直接选用模型并连接后端".into());
                    }
                    self.log(format!("安装独立后端到 {distro}:{ROOT}"));
                    let mut cmd = self.wsl(&distro);
                    self.deploy(&distro)?;
                    cmd.args(["setsid", "bash", "-s"]);
                    self.run(
                        cmd,
                        Some(
                            include_bytes!("../../../../../scripts/voice-backend-install.sh")
                                .to_vec(),
                        ),
                        Duration::from_secs(7200),
                    )?;
                    self.deploy(&distro)?;
                }
                "download_model" => {
                    let id = request.model_id.ok_or("请选择模型")?;
                    if !catalog().iter().any(|x| x.id == id) {
                        return Err("未知模型".into());
                    }
                    self.deploy(&distro)?;
                    let mut cmd = self.wsl(&distro);
                    cmd.args(["python3", CONTROLLER, "download", &id]);
                    self.log(format!("下载模型 {id}；可取消后重试，保留下载缓存"));
                    self.run(cmd, None, Duration::from_secs(14400))?;
                }
                "select_model" => {
                    self.probe(&distro)?;
                    let id = request.model_id.ok_or("请选择模型")?;
                    let mut s = self.state.lock().unwrap();
                    let model = s
                        .models
                        .iter()
                        .find(|x| x.id == id)
                        .ok_or("未知模型")?
                        .clone();
                    if !model.downloaded || model.capability == "download_only" {
                        return Err("模型未下载完整或尚未适配".into());
                    }
                    if s.inference_running {
                        return Err("请先停止推理再选择模型".into());
                    }
                    for item in &mut s.models {
                        if item.capability == model.capability {
                            item.selected = item.id == id;
                        }
                    }
                }
                "start_inference" => {
                    if !self
                        .snapshot()
                        .models
                        .iter()
                        .any(|x| x.id == "gpt-sovits-v2" && x.selected && x.downloaded)
                    {
                        return Err("请先下载并选择 GPT-SoVITS v2".into());
                    }
                    self.deploy(&distro)?;
                    let mut cmd = self.wsl(&distro);
                    cmd.args(["python3", CONTROLLER, "start"]);
                    self.run(cmd, None, Duration::from_secs(240))?;
                }
                "stop_inference" => {
                    self.stop_remote("stop")?;
                }
                _ => return Err("未知环境操作".into()),
            }
            self.probe(&distro)?;
            self.save()
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub use windows::EnvironmentManager;
#[cfg(not(target_os = "linux"))]
use windows::*;
#[cfg(all(test, not(target_os = "linux")))]
#[path = "tests.rs"]
mod tests;
