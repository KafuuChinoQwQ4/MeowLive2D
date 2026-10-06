//! App-owned server process: readiness, external-service reuse and parent-pipe shutdown.
use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct ServerStatus {
    pub stopped: bool,
    pub ready: bool,
    pub managed: bool,
    pub last_error: Option<String>,
    pub log_path: String,
}

pub struct ServerHandle {
    state: Arc<Mutex<ServerStatus>>,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ServerHandle {
    pub fn stopped(directory: PathBuf) -> Self {
        Self {
            state: Arc::new(Mutex::new(ServerStatus {
                stopped: true,
                ready: false,
                managed: false,
                last_error: None,
                log_path: directory.join("server.log").to_string_lossy().into_owned(),
            })),
            stopping: Arc::new(AtomicBool::new(true)),
            worker: None,
        }
    }

    pub fn start(
        executable: PathBuf,
        directory: PathBuf,
        origin: String,
        allow_local_fallback: bool,
    ) -> Result<Self, String> {
        Self::start_inner(executable, directory, origin, allow_local_fallback, None)
    }

    pub fn start_wsl(distro: String, directory: PathBuf, origin: String) -> Result<Self, String> {
        Self::start_inner(PathBuf::new(), directory, origin, false, Some(distro))
    }

    fn start_inner(
        executable: PathBuf,
        directory: PathBuf,
        origin: String,
        allow_local_fallback: bool,
        wsl_distro: Option<String>,
    ) -> Result<Self, String> {
        let state = Arc::new(Mutex::new(ServerStatus {
            stopped: false,
            ready: false,
            managed: false,
            last_error: None,
            log_path: directory.join("server.log").to_string_lossy().into_owned(),
        }));
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_state = state.clone();
        let worker_stop = stopping.clone();
        let worker = thread::Builder::new()
            .name("meowlive-server-host".into())
            .spawn(move || {
                if let Err(error) = supervise(
                    &executable,
                    &directory,
                    &origin,
                    allow_local_fallback,
                    wsl_distro.as_deref(),
                    &worker_stop,
                    &worker_state,
                ) {
                    let mut state = worker_state.lock().unwrap_or_else(|e| e.into_inner());
                    state.ready = false;
                    state.last_error = Some(error);
                }
            })
            .map_err(|error| format!("无法启动主服务管理线程：{error}"))?;
        Ok(Self {
            state,
            stopping,
            worker: Some(worker),
        })
    }

    pub fn status(&self) -> ServerStatus {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn shutdown_failed(&mut self, error: String) {
        self.shutdown();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.stopped = false;
        state.last_error = Some(error);
    }

    pub fn shutdown(&mut self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.ready = false;
        state.stopped = true;
        state.last_error = None;
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct OwnedServer(Child);
impl Drop for OwnedServer {
    fn drop(&mut self) {
        // EOF also arrives when the desktop crashes; the server shuts down gracefully.
        drop(self.0.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(8);
        while matches!(self.0.try_wait(), Ok(None)) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(30));
        }
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

fn healthy(client: &reqwest::blocking::Client, origin: &str) -> bool {
    let result = (|| {
        let response = client
            .get(format!("{origin}/api/health"))
            .send()
            .ok()?
            .error_for_status()
            .ok()?;
        let mut bytes = Vec::new();
        response.take(64 * 1024 + 1).read_to_end(&mut bytes).ok()?;
        if bytes.len() > 64 * 1024 {
            return None;
        }
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        Some(
            value["service"] == "meowlive"
                && value["protocol_version"] == meowlive_protocol::PROTOCOL_VERSION
                && value["bridge_connected"].is_boolean(),
        )
    })();
    result == Some(true)
}

fn should_start_owned_server(healthy: bool, local: bool, allow_local_fallback: bool) -> bool {
    !healthy && local && allow_local_fallback
}

fn should_start_wsl_server(healthy: bool, local: bool, has_distro: bool) -> bool {
    !healthy && local && has_distro
}

fn startup_failure(bytes: &[u8]) -> &'static str {
    let utf8 = String::from_utf8_lossy(bytes);
    let utf16 = String::from_utf16_lossy(
        &bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>(),
    );
    if utf8.contains("Wsl/Service/0x8007274c") || utf16.contains("Wsl/Service/0x8007274c") {
        "WSL 控制服务连接超时（Wsl/Service/0x8007274c）。请先保存 WSL 中的工作，在 Windows PowerShell 执行 wsl --shutdown，再重新打开 App 并开启主服务。诊断已显示在运行日志页。"
    } else if utf8.contains("PostgreSQL 启动失败") {
        "PostgreSQL 启动失败，请确认 Docker 已运行并检查 WSL 项目的数据库配置。诊断已显示在运行日志页。"
    } else if utf8.contains("找不到 WSL 中的 MeowLive2D") {
        "找不到选定 WSL 发行版中的 MeowLive2D 项目或启动器配置。诊断已显示在运行日志页。"
    } else {
        "WSL 主服务启动失败；请检查 WSL、项目依赖与数据库状态。诊断已显示在运行日志页。"
    }
}

// Keep nested shell/Python quotes out of the Windows command line. Source from
// a separate descriptor so stdin remains the server's parent-lifetime pipe.
fn wsl_script_argument(script: &str) -> String {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(script.as_bytes());
    format!("source <(printf %s {encoded}|base64 -d)")
}

fn wsl_server_script() -> &'static str {
    r#"for root in /root/code/MeowLive2D /root/MeowLive2D /home/*/code/MeowLive2D /home/*/MeowLive2D; do
  if [ -f "$root/scripts/rust_cache.py" ] && [ -f "$root/config/local/launcher.json" ]; then
    config=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("serverConfig", ""))' "$root/config/local/launcher.json") || exit 72
    case "$config" in /*) ;; *) config="$root/$config" ;; esac
    [ -f "$config" ] || continue
    cd "$root" || exit 72
    export MEOWLIVE_DESKTOP_PARENT=1
    exec node scripts/launcher/server.mjs --config "$config"
  fi
done
echo '找不到 WSL 中的 MeowLive2D 项目或启动器配置的主服务 TOML' >&2
exit 72"#
}

fn configuration(directory: &Path, address: &str) -> Result<PathBuf, String> {
    let folder = directory.join("server");
    fs::create_dir_all(&folder).map_err(|e| format!("无法创建主服务配置目录：{e}"))?;
    let path = folder.join("server.toml");
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => write!(
            file,
            "# Windows App 配置；保留本机修改。TTS 默认连接 127.0.0.1:9880。\n\
             [server]\nlisten_address = \"{address}\"\n\
             # 配置 PostgreSQL 后可启用观众档案；首次启动无需数据库。\n\
             [viewers]\nenabled = false\n"
        )
        .map_err(|e| format!("无法保存主服务配置：{e}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(format!("无法初始化主服务配置：{error}")),
    }
    Ok(path)
}

fn supervise(
    executable: &Path,
    directory: &Path,
    origin: &str,
    allow_local_fallback: bool,
    wsl_distro: Option<&str>,
    stopping: &AtomicBool,
    state: &Mutex<ServerStatus>,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(800))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let url = url::Url::parse(origin).map_err(|e| e.to_string())?;
    let local = url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    let mut child = None;
    let mut startup_log_offset = 0;
    let is_healthy = healthy(&client, origin);
    let start_wsl = should_start_wsl_server(is_healthy, local, wsl_distro.is_some());
    let start_local = should_start_owned_server(is_healthy, local, allow_local_fallback);
    if (start_wsl || start_local) && !stopping.load(Ordering::Acquire) {
        if let Some(distro) = wsl_distro.filter(|_| start_wsl) {
            let log = OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join("server.log"))
                .map_err(|e| format!("无法打开主服务日志：{e}"))?;
            startup_log_offset = log.metadata().map_err(|e| e.to_string())?.len();
            let mut command = Command::new("wsl.exe");
            command
                .args([
                    "-d", distro, "-u", "root", "--cd", "/", "--exec", "bash", "-lc",
                ])
                .arg(wsl_script_argument(wsl_server_script()))
                .stdin(Stdio::piped())
                .stdout(log.try_clone().map_err(|e| e.to_string())?)
                .stderr(log);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000);
            }
            child = Some(OwnedServer(
                command
                    .spawn()
                    .map_err(|e| format!("无法启动 WSL 主服务：{e}"))?,
            ));
            state.lock().unwrap_or_else(|e| e.into_inner()).managed = true;
        } else {
            let host = if url.host_str() == Some("[::1]") {
                "::1"
            } else {
                "127.0.0.1"
            };
            let address = (host, url.port_or_known_default().ok_or("主服务端口无效")?)
                .to_socket_addrs()
                .map_err(|e| e.to_string())?
                .next()
                .ok_or("主服务地址无效")?;
            if TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok() {
                return Err(
                    "主服务端口已被占用，且不是兼容的 MeowLive2D 服务。请释放端口后重新打开应用。"
                        .into(),
                );
            }
            let config = configuration(directory, &address.to_string())?;
            let log = OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join("server.log"))
                .map_err(|e| format!("无法打开主服务日志：{e}"))?;
            let mut command = Command::new(executable);
            command
                .arg("--config")
                .arg(config)
                .current_dir(directory)
                .env("MEOWLIVE_DESKTOP_PARENT", "1")
                .stdin(Stdio::piped())
                .stdout(log.try_clone().map_err(|e| e.to_string())?)
                .stderr(log);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            }
            child =
                Some(OwnedServer(command.spawn().map_err(|e| {
                    format!("无法启动主服务，请重新安装应用：{e}")
                })?));
            state.lock().unwrap_or_else(|e| e.into_inner()).managed = true;
        }
    }
    let deadline = Instant::now() + Duration::from_secs(if start_wsl { 180 } else { 30 });
    let mut was_ready = false;
    while !stopping.load(Ordering::Acquire) {
        if let Some(child) = &mut child {
            if let Some(exit) = child.0.try_wait().map_err(|e| e.to_string())? {
                let log_path = state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .log_path
                    .clone();
                return Err(if start_wsl {
                    let mut bytes = Vec::new();
                    if let Ok(mut log) = fs::File::open(&log_path) {
                        let _ = log.seek(SeekFrom::Start(startup_log_offset));
                        let _ = log.take(64 * 1024).read_to_end(&mut bytes);
                    }
                    format!("{}（{exit}）", startup_failure(&bytes))
                } else {
                    format!("主服务已退出（{exit}），请查看日志：{log_path}")
                });
            }
        }
        let ready = healthy(&client, origin);
        {
            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
            state.ready = ready;
            state.last_error = if !ready && was_ready {
                Some("主服务连接已断开，正在等待恢复。".into())
            } else {
                None
            };
        }
        was_ready |= ready;
        if !was_ready && Instant::now() > deadline {
            return Err(if start_wsl {
                "启动 WSL 主服务超时，请查看 Windows 服务日志并确认 WSL 项目配置有效。"
            } else if local && allow_local_fallback {
                "主服务启动超时，请检查日志后重新打开应用。"
            } else if local {
                "无法连接 WSL 主服务，且没有可用的 WSL 启动配置。请检查环境页中选定的发行版。"
            } else {
                "无法连接配置的远程主服务，请检查服务地址和网络。"
            }
            .into());
        }
        for _ in 0..5 {
            if stopping.load(Ordering::Acquire) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        should_start_owned_server, should_start_wsl_server, startup_failure, wsl_server_script,
    };

    #[test]
    fn authoritative_external_mode_never_starts_a_local_copy() {
        assert!(!should_start_owned_server(false, true, false));
        assert!(!should_start_owned_server(true, true, false));
        assert!(!should_start_owned_server(false, false, false));
    }

    #[test]
    fn standalone_local_mode_can_start_when_no_service_is_healthy() {
        assert!(should_start_owned_server(false, true, true));
        assert!(!should_start_owned_server(true, true, true));
        assert!(!should_start_owned_server(false, false, true));
    }

    #[test]
    fn wsl_server_start_uses_the_existing_linux_checkout_and_parent_pipe() {
        let script = wsl_server_script();
        assert!(script.contains("/root/code/MeowLive2D"));
        assert!(script.contains("/home/"));
        assert!(script.contains("config/local/launcher.json"));
        assert!(script.contains("serverConfig"));
        assert!(script.contains("MEOWLIVE_DESKTOP_PARENT=1"));
        assert!(script.contains("node scripts/launcher/server.mjs"));
        assert!(!script.contains("meowlive-server.exe"));
        assert!(should_start_wsl_server(false, true, true));
        assert!(!should_start_wsl_server(true, true, true));
        assert!(!should_start_wsl_server(false, false, true));
        assert!(!should_start_wsl_server(false, true, false));
    }
    #[test]
    fn utf16_wsl_timeout_is_reported_without_raw_log_contents() {
        let bytes: Vec<u8> = "private token\n错误代码: Wsl/Service/0x8007274c\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let message = startup_failure(&bytes);
        assert!(message.contains("Wsl/Service/0x8007274c"));
        assert!(message.contains("wsl --shutdown"));
        assert!(!message.contains("private"));
    }

    #[test]
    fn failed_start_cleanup_retains_error_for_control_panel_logs() {
        let mut handle = super::ServerHandle::stopped(std::env::temp_dir());
        handle.shutdown_failed("启动诊断".into());
        assert!(!handle.status().stopped);
        assert_eq!(handle.status().last_error.as_deref(), Some("启动诊断"));
    }
    #[cfg(unix)]
    #[test]
    fn encoded_script_preserves_quotes_unicode_and_parent_stdin() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let script = r#"exec python3 -c 'import json,sys; v=json.loads(sys.stdin.read()); print(v["路径"])'"#;
        let mut child = Command::new("bash")
            .args(["-lc", &super::wsl_script_argument(script)])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        input
            .write_all(r#"{"路径":"a space/人物.json"}"#.as_bytes())
            .unwrap();
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "a space/人物.json"
        );
    }

    #[test]
    fn configuration_syntax_error_is_not_reported_as_missing_project() {
        let message =
            startup_failure(b"SyntaxError: unexpected character after line continuation character");
        assert!(!message.contains("找不到"));
    }
}
