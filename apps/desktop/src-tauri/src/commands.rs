//! IPC 提供运行状态和固定依赖页面跳转；不暴露 VTS 令牌或服务商配置。

#[cfg(windows)]
pub struct DesktopState {
    pub closing: std::sync::atomic::AtomicBool,
    pub environment: std::sync::Arc<crate::environment::EnvironmentManager>,
    pub updates: std::sync::Arc<crate::updates::UpdateManager>,
    pub maintenance: std::sync::Mutex<()>,
    pub server: std::sync::Mutex<crate::managed_server::ServerHandle>,
    pub runtime: std::sync::Mutex<meowlive_desktop_runtime::host::RuntimeHandle>,
    pub config_path: String,
    pub server_url: String,
}

#[cfg(windows)]
#[derive(serde::Serialize)]
pub struct DesktopStatus {
    server: crate::managed_server::ServerStatus,
    config_path: String,
    server_url: String,
    runtime: meowlive_desktop_runtime::host::RuntimeStatus,
}

#[cfg(windows)]
#[tauri::command]
pub async fn desktop_status(app: tauri::AppHandle) -> Result<DesktopStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<DesktopState>();
        DesktopStatus {
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
        }
    })
    .await
    .map_err(|e| e.to_string())
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
