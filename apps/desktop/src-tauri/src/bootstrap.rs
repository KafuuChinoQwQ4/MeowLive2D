//! 窗口生命周期只负责组装、启动和停止独立执行库。

#[cfg(not(windows))]
pub fn run() -> Result<(), String> {
    Err("The Tauri desktop shell requires Windows. Use meowlive-client --config PATH --simulate for Linux validation.".into())
}

#[cfg(windows)]
pub fn run() -> Result<(), String> {
    use crate::{
        commands::DesktopState,
        managed_server::ServerHandle,
        startup::{load_configuration, parse_arguments},
    };
    use meowlive_desktop_runtime::host::RuntimeHandle;
    use std::sync::{Arc, Mutex};
    use tauri::{Manager, RunEvent};

    let options = parse_arguments(std::env::args().skip(1))?;
    if options.help {
        println!(
            "MeowLive2D desktop\nUsage: meowlive-desktop [--config PATH] [--simulate]\nWithout --config, desktop.toml is created in the application configuration directory."
        );
        return Ok(());
    }
    let application = tauri::Builder::default()
        .setup(move |app| {
            let loaded = load_configuration(
                options.config_path.as_deref(),
                &app.path().app_config_dir()?,
            )
            .map_err(std::io::Error::other)?;
            let executable = std::env::current_exe()?.with_file_name("meowlive-server.exe");
            let server = ServerHandle::start(
                executable,
                loaded
                    .config_path
                    .parent()
                    .ok_or_else(|| std::io::Error::other("配置目录不存在"))?
                    .to_owned(),
                loaded.server_url.clone(),
            )
            .map_err(std::io::Error::other)?;
            let runtime = RuntimeHandle::start(loaded.config, options.simulation)
                .map_err(std::io::Error::other)?;
            let data_dir = loaded
                .config_path
                .parent()
                .ok_or("配置目录不存在")?
                .to_owned();
            let environment = Arc::new(crate::environment::EnvironmentManager::new(
                data_dir.join("desktop-environment.json"),
            ));
            let updates = crate::updates::UpdateManager::new(data_dir.join("updates"));
            let _ = environment.action(crate::environment::EnvironmentRequest {
                action: "detect".into(),
                distro: None,
                model_id: None,
            });
            app.manage(DesktopState {
                closing: std::sync::atomic::AtomicBool::new(false),
                environment,
                updates,
                maintenance: Mutex::new(()),
                server: Mutex::new(server),
                runtime: Mutex::new(runtime),
                config_path: loaded.config_path.to_string_lossy().into_owned(),
                server_url: loaded.server_url,
            });
            let resume_app = app.handle().clone();
            std::thread::spawn(move || {
                // Detection and server startup are asynchronous; reconnect only an idle,
                // previously selected backend, without loading models during training.
                for _ in 0..120 {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let state = resume_app.state::<DesktopState>();
                    if state.closing.load(std::sync::atomic::Ordering::Acquire) {
                        break;
                    }
                    let snapshot = state.environment.snapshot();
                    if snapshot.busy {
                        continue;
                    }
                    if !snapshot.backend.ready || snapshot.inference_running {
                        break;
                    }
                    if !snapshot
                        .models
                        .iter()
                        .any(|m| m.id == "gpt-sovits-v2" && m.selected && m.downloaded)
                    {
                        break;
                    }
                    if !state
                        .server
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .status()
                        .ready
                    {
                        continue;
                    }
                    let _guard = state.maintenance.lock().unwrap_or_else(|e| e.into_inner());
                    if !state.closing.load(std::sync::atomic::Ordering::Acquire)
                        && crate::maintenance::require_idle(&state).is_ok()
                    {
                        let _ = state
                            .environment
                            .action(crate::environment::EnvironmentRequest {
                                action: "start_inference".into(),
                                distro: snapshot.selected_distro,
                                model_id: None,
                            });
                    }
                    break;
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if let Some(state) = window.try_state::<DesktopState>() {
                    use tauri::Emitter;
                    let guard = state.maintenance.try_lock();
                    if state.environment.snapshot().busy || guard.is_err() {
                        api.prevent_close();
                        let _ = window.emit(
                            "desktop-exit-blocked",
                            "环境或维护任务正在运行，请先取消或等待完成，再关闭 App。",
                        );
                        return;
                    }
                    if let Err(error) = state.environment.shutdown() {
                        api.prevent_close();
                        let _ = window.emit(
                            "desktop-exit-blocked",
                            format!("语音后端未能停止，请检查环境页后重试：{error}"),
                        );
                        return;
                    }
                    state
                        .closing
                        .store(true, std::sync::atomic::Ordering::Release);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::desktop_status,
            crate::commands::open_dependency_page,
            crate::commands::environment_status,
            crate::commands::environment_action,
            crate::commands::environment_apply,
            crate::commands::update_status,
            crate::commands::update_check,
            crate::commands::update_prepare,
            crate::commands::update_cancel,
            crate::commands::update_install
        ])
        .build(tauri::generate_context!())
        .map_err(|error| error.to_string())?;
    application.run(|app, event| {
        if let RunEvent::ExitRequested { api, .. } = &event {
            if let Some(state) = app.try_state::<DesktopState>() {
                if state.environment.snapshot().busy || state.maintenance.try_lock().is_err() {
                    api.prevent_exit();
                    use tauri::Emitter;
                    let _ = app.emit(
                        "desktop-exit-blocked",
                        "环境任务正在运行，请先在环境与模型页取消任务或等待完成，再关闭 App。",
                    );
                    return;
                }
            }
        }
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            if let Some(state) = app.try_state::<DesktopState>() {
                state
                    .closing
                    .store(true, std::sync::atomic::Ordering::Release);
                let _ = state.updates.cancel();
                if let Err(error) = state.environment.shutdown() {
                    eprintln!("voice backend shutdown failed: {error}");
                }
                if let Err(error) = state
                    .runtime
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .shutdown()
                {
                    eprintln!("desktop shutdown failed: {error}");
                }
                state
                    .server
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .shutdown();
            }
        }
    });
    Ok(())
}
