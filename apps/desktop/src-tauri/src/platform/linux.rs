//! Native Linux shell state and IPC.
use crate::managed_server::{ServerHandle, ServerStatus};
use std::sync::Mutex;

pub struct LinuxState {
    pub browser_panel: Option<crate::browser_panel::BrowserPanelHandle>,
    pub browser_panel_error: Option<String>,
    pub server: Mutex<ServerHandle>,
    pub environment: std::sync::Arc<crate::environment::EnvironmentManager>,
    pub updates: std::sync::Arc<crate::updates::UpdateManager>,
    pub maintenance: Mutex<()>,
    pub config_path: String,
    pub server_url: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct LinuxDesktopStatus {
    pub platform: &'static str,
    pub browser_panel_error: Option<String>,
    pub config_path: String,
    pub server_url: String,
    pub server: ServerStatus,
    pub environment: meowlive_protocol::desktop_environment::EnvironmentSnapshot,
}

pub fn dependency_url(id: &str) -> Result<&'static str, &'static str> {
    match id {
        "docker" => Ok("https://docs.docker.com/desktop/setup/install/linux-install/"),
        "postgres" => Ok("https://www.postgresql.org/download/"),
        "pgvector" => Ok("https://github.com/pgvector/pgvector#docker"),
        "neo4j" => Ok("https://neo4j.com/deployment-center/"),
        "project" => Ok("https://github.com/KafuuChinoQwQ4/MeowLive2D/archive/refs/heads/main.zip"),
        _ => Err("未知的依赖下载来源"),
    }
}

pub fn status(state: &LinuxState) -> LinuxDesktopStatus {
    LinuxDesktopStatus {
        platform: "linux",
        browser_panel_error: state.browser_panel_error.clone(),
        config_path: state.config_path.clone(),
        server_url: state.server_url.clone(),
        server: state
            .server
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status(),
        environment: state.environment.snapshot(),
    }
}

#[tauri::command]
pub fn desktop_status(state: tauri::State<'_, LinuxState>) -> LinuxDesktopStatus {
    status(&state)
}

#[tauri::command]
pub fn desktop_service_set_enabled(
    state: tauri::State<'_, LinuxState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    match id.as_str() {
        "server" => {
            let _guard = state
                .maintenance
                .try_lock()
                .map_err(|_| "其他维护任务正在运行，请稍后重试")?;
            let mut server = state.server.lock().unwrap_or_else(|e| e.into_inner());
            if enabled {
                if !server.status().stopped && server.status().ready {
                    Ok(())
                } else {
                    let config = std::path::Path::new(&state.config_path);
                    let directory = config.parent().ok_or("主服务配置目录无效")?.to_owned();
                    let executable = std::env::current_exe()
                        .map_err(|error| format!("无法解析 Linux App 路径：{error}"))?
                        .with_file_name("meowlive-server");
                    *server =
                        ServerHandle::start(executable, directory, state.server_url.clone(), true)?;
                    Ok(())
                }
            } else {
                server.shutdown();
                Ok(())
            }
        }
        "tts" => {
            if enabled
                && !state
                    .server
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .status()
                    .ready
            {
                return Err("请先在“启动与运行”中手动启动主服务".into());
            }
            let snapshot = state.environment.snapshot();
            if snapshot.inference_running == enabled {
                Ok(())
            } else {
                let action = if enabled {
                    "start_inference"
                } else {
                    "stop_inference"
                };
                state
                    .environment
                    .clone()
                    .action(crate::environment::EnvironmentRequest {
                        action: action.into(),
                        distro: None,
                        model_id: None,
                        config_path: None,
                    })
                    .map(|_| ())
            }
        }
        "windows" => Err("Linux App 不支持 Windows 执行端".into()),
        _ => Err("未知服务".into()),
    }
}

#[tauri::command]
pub fn environment_status(
    state: tauri::State<'_, LinuxState>,
) -> crate::environment::EnvironmentSnapshot {
    state.environment.snapshot()
}

#[tauri::command]
pub async fn environment_action(
    state: tauri::State<'_, LinuxState>,
    request: crate::environment::EnvironmentRequest,
) -> Result<crate::environment::EnvironmentSnapshot, String> {
    if request.action == "start_inference"
        && !state
            .server
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status()
            .ready
    {
        return Err("请先在“启动与运行”中手动启动主服务".into());
    }
    state.environment.clone().action(request)
}

#[tauri::command]
pub fn environment_apply() -> Result<(), String> {
    Err("Linux 本机语音环境无需应用 WSL 配置".into())
}

#[tauri::command]
pub fn update_status(state: tauri::State<'_, LinuxState>) -> crate::updates::UpdateStatus {
    state.updates.status()
}

#[tauri::command]
pub fn update_check(state: tauri::State<'_, LinuxState>) -> Result<(), String> {
    state.updates.check()
}

#[tauri::command]
pub fn update_prepare(state: tauri::State<'_, LinuxState>) -> Result<(), String> {
    let _guard = state
        .maintenance
        .try_lock()
        .map_err(|_| "其他维护任务正在运行，请稍后重试")?;
    if state.environment.snapshot().busy {
        return Err("环境任务正在运行，请等待完成".into());
    }
    state.updates.prepare()
}

#[tauri::command]
pub fn update_cancel(state: tauri::State<'_, LinuxState>) -> Result<(), String> {
    state.updates.cancel()
}

#[tauri::command]
pub async fn update_install(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<LinuxState>();
        let _guard = state
            .maintenance
            .try_lock()
            .map_err(|_| "其他维护任务正在运行，请稍后重试")?;
        if state.environment.snapshot().busy {
            return Err("环境任务正在运行，请等待完成".into());
        }
        if state.updates.status().phase != "ready" {
            return Err("请先下载并校验更新".into());
        }
        state.updates.launch_installer()?;
        drop(_guard);
        app.exit(0);
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn open_dependency_page(id: String) -> Result<(), String> {
    open_url(dependency_url(&id).map_err(str::to_owned)?)
}

#[tauri::command]
pub fn open_control_panel() -> Result<(), String> {
    open_url("http://127.0.0.1:1420")
}

pub fn open_url(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|_| "无法打开系统浏览器，请确认已安装 xdg-open。".into())
}

#[cfg(test)]
mod tests {
    use super::{LinuxDesktopStatus, dependency_url};

    #[test]
    fn linux_dependency_links_are_fixed_and_unknown_ids_are_rejected() {
        assert!(dependency_url("docker").unwrap().contains("linux-install"));
        assert_eq!(dependency_url("arbitrary://url"), Err("未知的依赖下载来源"));
    }

    #[test]
    fn status_contract_identifies_linux_shell_and_voice_limit() {
        let status = LinuxDesktopStatus {
            platform: "linux",
            browser_panel_error: None,
            config_path: "/tmp/desktop.toml".into(),
            server_url: "http://127.0.0.1:1420".into(),
            server: crate::managed_server::ServerStatus {
                stopped: false,
                ready: true,
                managed: true,
                last_error: None,
                log_path: "/tmp/server.log".into(),
            },
            environment: crate::environment::EnvironmentSnapshot {
                phase: "unsupported".into(),
                busy: false,
                message: String::new(),
                logs: vec![],
                distros: vec![],
                selected_distro: None,
                backend: Default::default(),
                models: vec![],
                progress: 0,
                inference_running: false,
            },
        };
        let value = serde_json::to_value(status).unwrap();
        assert_eq!(value["platform"], "linux");
        assert_eq!(value["server"]["ready"], true);
        assert_eq!(value["environment"]["phase"], "unsupported");
    }
}
