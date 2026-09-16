//! 窗口生命周期只负责组装、启动和停止独立执行库。

#[cfg(not(windows))]
pub fn run() -> Result<(), String> {
    Err("The Tauri desktop shell requires Windows. Use meowlive-client --config PATH --simulate for Linux validation.".into())
}

#[cfg(windows)]
pub fn run() -> Result<(), String> {
    use crate::{
        commands::{DesktopState, desktop_status},
        startup::{load_configuration, parse_arguments},
    };
    use meowlive_desktop_runtime::host::RuntimeHandle;
    use std::sync::Mutex;
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
            let runtime = RuntimeHandle::start(loaded.config, options.simulation)
                .map_err(std::io::Error::other)?;
            app.manage(DesktopState {
                runtime: Mutex::new(runtime),
                config_path: loaded.config_path.to_string_lossy().into_owned(),
                server_url: loaded.server_url,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![desktop_status])
        .build(tauri::generate_context!())
        .map_err(|error| error.to_string())?;
    application.run(|app, event| {
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            if let Some(state) = app.try_state::<DesktopState>() {
                if let Err(error) = state
                    .runtime
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .shutdown()
                {
                    eprintln!("desktop shutdown failed: {error}");
                }
            }
        }
    });
    Ok(())
}
