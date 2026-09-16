//! Windows 执行库：与 Tauri 和 React 解耦，生命周期由桌面进程管理。

#[cfg(test)]
extern crate self as meowlive_desktop_runtime;

pub mod assets;
pub mod audio;
pub mod avatar;
pub mod cli;
pub mod config;
pub mod connection;
pub mod host;
pub mod lip_sync;
pub mod obs;
pub mod playback;
pub mod presentation;

mod resource_control;
