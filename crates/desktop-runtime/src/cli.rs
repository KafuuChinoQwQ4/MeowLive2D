//! 与 Tauri 生命周期分离的命令行启动、配置和重连入口。

use crate::{audio::DeviceBackend, config::ClientConfig};
use std::{env, fs, path::Path};
use tokio::time::Duration;

pub fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut config_path = None;
    let mut simulate = false;
    let mut once = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "MeowLive2D desktop execution client\nUsage: meowlive-client --config PATH [--simulate] [--once]\n  --simulate  Explicit silent simulation; no audio device is tested\n  --once      Exit after the first connection ends\n"
                );
                return Ok(());
            }
            "--config" => {
                config_path = Some(args.next().ok_or("--config requires a TOML path")?);
            }
            "--simulate" => simulate = true,
            "--once" => once = true,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let path = config_path.ok_or("--config PATH is required; use --help")?;
    let mut config = ClientConfig::from_toml(
        &fs::read_to_string(&path).map_err(|error| format!("cannot read {path}: {error}"))?,
    )?;
    config.resolve_paths(Path::new(&path));
    if simulate {
        eprintln!("SIMULATION: silent timing backend enabled; this does not test an audio device");
    }
    // Fail immediately on unsupported platforms even when the server is offline.
    if !simulate {
        let _ = DeviceBackend::new(config.max_buffer_samples)?;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    let result = runtime.block_on(crate::host::run_client(config, simulate, once, async {
        tokio::signal::ctrl_c()
            .await
            .map_err(|error| error.to_string())
    }));
    // A failed disk must not keep the CLI alive indefinitely after audio stops.
    runtime.shutdown_timeout(Duration::from_millis(250));
    result
}
