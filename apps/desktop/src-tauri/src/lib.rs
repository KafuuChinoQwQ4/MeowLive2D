//! Windows Tauri 薄外壳；执行逻辑与线程由独立 desktop-runtime 承担。

mod bootstrap;
mod commands;
pub mod managed_server;
pub mod startup;
pub mod updates;

pub fn run() -> Result<(), String> {
    bootstrap::run()
}

pub mod environment;

pub mod environment_config;

#[cfg(any(windows, target_os = "linux", test))]
pub mod browser_panel;

mod maintenance;

#[cfg(target_os = "linux")]
pub mod platform;
