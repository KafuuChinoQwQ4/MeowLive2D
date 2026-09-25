//! Windows Tauri 薄外壳；执行逻辑与线程由独立 desktop-runtime 承担。

mod bootstrap;
mod commands;
pub mod managed_server;
pub mod startup;

pub fn run() -> Result<(), String> {
    bootstrap::run()
}
