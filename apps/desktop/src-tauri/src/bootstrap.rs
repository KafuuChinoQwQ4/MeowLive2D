//! 窗口生命周期只负责组装、启动和停止独立执行库。

#[cfg(all(not(windows), not(target_os = "linux")))]
pub fn run() -> Result<(), String> {
    Err("The Tauri desktop shell requires Windows. Use meowlive-client --config PATH --simulate for Linux validation.".into())
}

#[cfg(target_os = "linux")]
pub fn run() -> Result<(), String> {
    use crate::{
        managed_server::ServerHandle,
        platform::linux::LinuxState,
        startup::{load_configuration, parse_arguments},
    };
    use tauri::Manager;

    let options = parse_arguments(std::env::args().skip(1))?;
    if options.help {
        println!(
            "MeowLive2D desktop\nUsage: meowlive-desktop [--config PATH]\nWithout --config, desktop.toml is created in the application configuration directory."
        );
        return Ok(());
    }
    let application = tauri::Builder::default()
        .setup(move |app| {
            let show =
                tauri::menu::MenuItem::with_id(app, "show-main", "显示窗口", true, None::<&str>)?;
            let quit =
                tauri::menu::MenuItem::with_id(app, "quit", "退出 MeowLive2D", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show, &quit])?;
            tauri::tray::TrayIconBuilder::with_id("meowlive-tray")
                .icon(app.default_window_icon().ok_or("应用图标不可用")?.clone())
                .tooltip("MeowLive2D")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "show-main" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    } else if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;
            let loaded = load_configuration(
                options.config_path.as_deref(),
                &app.path().app_config_dir()?,
            )
            .map_err(std::io::Error::other)?;
            let server = ServerHandle::stopped(
                loaded
                    .config_path
                    .parent()
                    .ok_or_else(|| std::io::Error::other("配置目录不存在"))?
                    .to_owned(),
            );
            let (browser_panel, browser_panel_error) =
                match crate::browser_panel::BrowserPanelHandle::start(
                    &app.path().resource_dir()?.join("control-panel"),
                    app.handle().clone(),
                ) {
                    Ok(panel) => (Some(panel), None),
                    Err(error) => (None, Some(error)),
                };
            app.manage(LinuxState {
                browser_panel,
                browser_panel_error,
                server: std::sync::Mutex::new(server),
                environment: std::sync::Arc::new(crate::environment::EnvironmentManager::new(
                    loaded
                        .config_path
                        .parent()
                        .ok_or_else(|| std::io::Error::other("配置目录不存在"))?
                        .join("desktop-environment.json"),
                )),
                updates: crate::updates::UpdateManager::new(app.path().app_cache_dir()?),
                maintenance: std::sync::Mutex::new(()),
                config_path: loaded.config_path.to_string_lossy().into_owned(),
                server_url: loaded.server_url,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::platform::linux::desktop_status,
            crate::platform::linux::desktop_service_set_enabled,
            crate::platform::linux::open_dependency_page,
            crate::platform::linux::open_control_panel,
            crate::platform::linux::environment_status,
            crate::platform::linux::environment_action,
            crate::platform::linux::environment_apply,
            crate::platform::linux::update_status,
            crate::platform::linux::update_check,
            crate::platform::linux::update_prepare,
            crate::platform::linux::update_cancel,
            crate::platform::linux::update_install
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if let Some(state) = window.try_state::<LinuxState>() {
                    if state.environment.snapshot().busy || state.maintenance.try_lock().is_err() {
                        api.prevent_close();
                        use tauri::Emitter;
                        let _ = window.emit(
                            "desktop-exit-blocked",
                            "环境任务正在运行，请先取消任务或等待完成，再关闭 App。",
                        );
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .map_err(|error| error.to_string())?;
    application.run(|app, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            if let Some(state) = app.try_state::<LinuxState>() {
                if let Some(panel) = &state.browser_panel {
                    panel.shutdown();
                }
                state
                    .server
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .shutdown();
                if let Err(error) = state.environment.shutdown() {
                    eprintln!("Linux voice backend shutdown failed: {error}");
                }
            }
        }
    });
    Ok(())
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
            let show =
                tauri::menu::MenuItem::with_id(app, "show-main", "显示窗口", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show])?;
            tauri::tray::TrayIconBuilder::with_id("meowlive-tray")
                .icon(app.default_window_icon().ok_or("应用图标不可用")?.clone())
                .tooltip("MeowLive2D")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "show-main" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
            let loaded = load_configuration(
                options.config_path.as_deref(),
                &app.path().app_config_dir()?,
            )
            .map_err(std::io::Error::other)?;
            let server_directory = loaded
                .config_path
                .parent()
                .ok_or("配置目录不存在")?
                .to_owned();
            let server = ServerHandle::stopped(server_directory);
            let runtime = RuntimeHandle::stopped(options.simulation);
            let data_dir = loaded
                .config_path
                .parent()
                .ok_or("配置目录不存在")?
                .to_owned();
            let environment = Arc::new(crate::environment::EnvironmentManager::new(
                data_dir.join("desktop-environment.json"),
            ));
            let updates = crate::updates::UpdateManager::new(data_dir.join("updates"));
            let (browser_panel, browser_panel_error) =
                match crate::browser_panel::BrowserPanelHandle::start(
                    &app.path().resource_dir()?.join("control-panel"),
                    app.handle().clone(),
                ) {
                    Ok(panel) => (Some(panel), None),
                    Err(error) => {
                        crate::startup::record_startup_error(&error);
                        (None, Some(error))
                    }
                };
            environment.validate_cached_state();
            app.manage(DesktopState {
                closing: std::sync::atomic::AtomicBool::new(false),
                auto_inference: std::sync::atomic::AtomicBool::new(false),
                environment,
                updates,
                maintenance: Mutex::new(()),
                server: Mutex::new(server),
                browser_panel,
                browser_panel_error,
                runtime: Mutex::new(runtime),
                runtime_config: loaded.config,
                config_path: loaded.config_path.to_string_lossy().into_owned(),
                server_url: loaded.server_url,
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
            crate::commands::desktop_service_set_enabled,
            crate::commands::open_dependency_page,
            crate::commands::open_control_panel,
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
                if let Some(panel) = &state.browser_panel {
                    panel.shutdown();
                }
            }
        }
    });
    Ok(())
}
