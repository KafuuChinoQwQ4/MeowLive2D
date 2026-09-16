//! IPC 仅公开宿主状态和面板所需地址；不暴露 VTS 令牌或服务商配置。

#[cfg(windows)]
pub struct DesktopState {
    pub runtime: std::sync::Mutex<meowlive_desktop_runtime::host::RuntimeHandle>,
    pub config_path: String,
    pub server_url: String,
}

#[cfg(windows)]
#[derive(serde::Serialize)]
pub struct DesktopStatus {
    config_path: String,
    server_url: String,
    runtime: meowlive_desktop_runtime::host::RuntimeStatus,
}

#[cfg(windows)]
#[tauri::command]
pub fn desktop_status(state: tauri::State<'_, DesktopState>) -> DesktopStatus {
    DesktopStatus {
        config_path: state.config_path.clone(),
        server_url: state.server_url.clone(),
        runtime: state
            .runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status(),
    }
}
