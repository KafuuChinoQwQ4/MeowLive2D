//! IPC 提供运行状态和固定依赖页面跳转；不暴露 VTS 令牌或服务商配置。

#[cfg(windows)]
pub struct DesktopState {
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
pub fn desktop_status(state: tauri::State<'_, DesktopState>) -> DesktopStatus {
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
