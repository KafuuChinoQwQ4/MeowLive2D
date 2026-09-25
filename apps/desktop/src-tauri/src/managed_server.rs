//! App-owned server process: readiness, external-service reuse and parent-pipe shutdown.
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
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
    pub fn start(executable: PathBuf, directory: PathBuf, origin: String) -> Result<Self, String> {
        let state = Arc::new(Mutex::new(ServerStatus {
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

    pub fn shutdown(&mut self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.state.lock().unwrap_or_else(|e| e.into_inner()).ready = false;
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
    if !healthy(&client, origin) && local && !stopping.load(Ordering::Acquire) {
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
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut was_ready = false;
    while !stopping.load(Ordering::Acquire) {
        if let Some(child) = &mut child {
            if let Some(exit) = child.0.try_wait().map_err(|e| e.to_string())? {
                return Err(format!(
                    "主服务已退出（{exit}），请查看日志、处理错误后重新打开应用。"
                ));
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
            return Err(if local {
                "主服务启动超时，请检查日志后重新打开应用。"
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
