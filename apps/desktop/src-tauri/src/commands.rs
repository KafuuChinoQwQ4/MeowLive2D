//! IPC 提供运行状态和固定依赖页面跳转；不暴露 VTS 令牌或服务商配置。

#[cfg(windows)]
pub struct DesktopState {
    pub closing: std::sync::atomic::AtomicBool,
    pub auto_inference: std::sync::atomic::AtomicBool,
    pub environment: std::sync::Arc<crate::environment::EnvironmentManager>,
    pub updates: std::sync::Arc<crate::updates::UpdateManager>,
    pub maintenance: std::sync::Mutex<()>,
    pub server: std::sync::Mutex<crate::managed_server::ServerHandle>,
    pub browser_panel: Option<crate::browser_panel::BrowserPanelHandle>,
    pub browser_panel_error: Option<String>,
    pub runtime: std::sync::Mutex<meowlive_desktop_runtime::host::RuntimeHandle>,
    pub runtime_config: meowlive_desktop_runtime::config::ClientConfig,
    pub config_path: String,
    pub server_url: String,
}

#[cfg(windows)]
#[derive(serde::Serialize)]
pub struct DesktopStatus {
    platform: &'static str,
    environment: crate::environment::EnvironmentSnapshot,
    server: crate::managed_server::ServerStatus,
    config_path: String,
    server_url: String,
    runtime: meowlive_desktop_runtime::host::RuntimeStatus,
    browser_panel_error: Option<String>,
}

#[cfg(windows)]
#[tauri::command]
pub async fn desktop_status(app: tauri::AppHandle) -> Result<DesktopStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<DesktopState>();
        DesktopStatus {
            platform: "windows",
            environment: state.environment.snapshot(),
            server: state
                .server
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .status(),
            config_path: state.config_path.clone(),
            server_url: state.server_url.clone(),
            runtime: state
                .runtime
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .status(),
            browser_panel_error: state.browser_panel_error.clone(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(windows)]
#[tauri::command]
pub async fn desktop_service_set_enabled(
    app: tauri::AppHandle,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<DesktopState>();
        if !matches!(id.as_str(), "server" | "tts" | "windows") {
            return Err("未知服务".into());
        }
        if state.closing.load(std::sync::atomic::Ordering::Acquire) {
            return Err("App 正在关闭".into());
        }
        let _guard = state
            .maintenance
            .try_lock()
            .map_err(|_| "其他维护任务正在运行，请稍后重试")?;
        let environment = state.environment.snapshot();
        if environment.busy || state.updates.is_busy() {
            return Err("环境或更新任务正在运行，请等待完成".into());
        }
        let server = state
            .server
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status();
        if id == "server" && server.ready && !server.managed {
            return Err("当前连接外部主服务，请在原启动程序中管理".into());
        }
        if server.ready && (!enabled || id == "tts") {
            crate::maintenance::require_idle(&state)?;
        }
        if id == "tts" || id == "server" && !enabled {
            state
                .auto_inference
                .store(false, std::sync::atomic::Ordering::Release);
        }
        match (id.as_str(), enabled) {
            ("server", true) if !server.ready => crate::maintenance::start_server(&state, false),
            ("server", false) => {
                state
                    .server
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .shutdown();
                Ok(())
            }
            ("windows", true) => {
                if !server.ready {
                    return Err("请先启动主服务".into());
                }
                let mut runtime = state.runtime.lock().unwrap_or_else(|e| e.into_inner());
                let status = runtime.status();
                if !status.running {
                    runtime.shutdown()?;
                    *runtime = meowlive_desktop_runtime::host::RuntimeHandle::start(
                        state.runtime_config.clone(),
                        status.simulation,
                    )?;
                }
                Ok(())
            }
            ("windows", false) => state
                .runtime
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .shutdown(),
            ("tts", _) => {
                if enabled && !server.ready {
                    return Err("请先启动主服务".into());
                }
                state
                    .environment
                    .action(crate::environment::EnvironmentRequest {
                        action: if enabled {
                            "start_inference"
                        } else {
                            "stop_inference"
                        }
                        .into(),
                        distro: environment.selected_distro,
                        model_id: None,
                        config_path: None,
                    })?;
                Ok(())
            }
            _ => Ok(()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(windows)]
#[tauri::command]
pub fn open_dependency_page(id: String) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let url = crate::startup::dependency_url(&id)?;
    std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .creation_flags(0x0800_0000)
        .spawn()
        .map(|_| ())
        .map_err(|_| "无法打开系统浏览器，请复制下载地址到浏览器。".into())
}

#[cfg(windows)]
#[tauri::command]
pub fn open_control_panel() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", "http://127.0.0.1:1420"])
        .creation_flags(0x0800_0000)
        .spawn()
        .map(|_| ())
        .map_err(|_| "无法打开系统浏览器，请确认浏览器已安装。".into())
}

#[cfg(windows)]
#[tauri::command]
pub fn environment_status(
    state: tauri::State<'_, DesktopState>,
) -> crate::environment::EnvironmentSnapshot {
    state.environment.snapshot()
}
#[cfg(windows)]
#[tauri::command]
pub async fn environment_action(
    app: tauri::AppHandle,
    request: crate::environment::EnvironmentRequest,
) -> Result<crate::environment::EnvironmentSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<DesktopState>();
        if state.closing.load(std::sync::atomic::Ordering::Acquire) {
            return Err("App 正在关闭".into());
        }
        let _guard = state
            .maintenance
            .try_lock()
            .map_err(|_| "其他维护任务正在运行，请稍后重试")?;
        if state.updates.is_busy() {
            return Err("更新任务正在运行".into());
        }
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
        if !matches!(
            request.action.as_str(),
            "detect" | "cancel" | "download_model" | "install_wsl"
        ) {
            crate::maintenance::require_idle(&state)?;
        }
        state.environment.action(request)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[cfg(windows)]
#[tauri::command]
pub async fn environment_apply(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        crate::maintenance::apply_environment(&app.state::<DesktopState>())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[cfg(windows)]
#[tauri::command]
pub fn update_status(state: tauri::State<'_, DesktopState>) -> crate::updates::UpdateStatus {
    state.updates.status()
}
#[cfg(windows)]
#[tauri::command]
pub fn update_check(state: tauri::State<'_, DesktopState>) -> Result<(), String> {
    state.updates.check()
}
#[cfg(windows)]
#[tauri::command]
pub fn update_prepare(state: tauri::State<'_, DesktopState>) -> Result<(), String> {
    let _guard = state
        .maintenance
        .try_lock()
        .map_err(|_| "其他维护任务正在运行，请稍后重试")?;
    if state.environment.snapshot().busy {
        return Err("环境任务正在运行，请等待完成".into());
    }
    state.updates.prepare()
}
#[cfg(windows)]
#[tauri::command]
pub fn update_cancel(state: tauri::State<'_, DesktopState>) -> Result<(), String> {
    state.updates.cancel()
}
#[cfg(windows)]
#[tauri::command]
pub async fn update_install(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<DesktopState>();
        if state.closing.load(std::sync::atomic::Ordering::Acquire) {
            return Err("App 正在关闭".into());
        }
        let _guard = state
            .maintenance
            .try_lock()
            .map_err(|_| "其他维护任务正在运行，请稍后重试")?;
        if state.environment.snapshot().busy {
            return Err("环境任务正在运行，请等待完成".into());
        }
        crate::maintenance::require_idle(&state)?;
        if state.updates.status().phase != "ready" {
            return Err("请先下载并校验更新".into());
        }
        state.environment.shutdown()?;
        state
            .server
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown();
        if let Err(error) = state.updates.launch_installer() {
            let _ = crate::maintenance::restart_server(&state);
            return Err(error);
        }
        state
            .closing
            .store(true, std::sync::atomic::Ordering::Release);
        drop(_guard);
        app.exit(0);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
