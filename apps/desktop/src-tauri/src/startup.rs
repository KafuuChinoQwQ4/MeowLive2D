//! 桌面启动参数与用户配置目录；不依赖窗口框架，便于跨平台验证。

use meowlive_desktop_runtime::config::ClientConfig;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const DEFAULT_CONFIG: &str = include_str!("../../../../config/desktop.example.toml");

#[derive(Default)]
pub struct StartupOptions {
    pub config_path: Option<PathBuf>,
    pub simulation: bool,
    pub help: bool,
}

pub struct LoadedConfiguration {
    pub config: ClientConfig,
    pub config_path: PathBuf,
    pub server_url: String,
}

pub fn parse_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> Result<StartupOptions, String> {
    let mut result = StartupOptions::default();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--config" if result.config_path.is_none() => {
                let path = arguments
                    .next()
                    .filter(|path| !path.is_empty() && !path.starts_with('-'))
                    .ok_or("--config requires a TOML path")?;
                result.config_path = Some(path.into());
            }
            "--simulate" if !result.simulation => result.simulation = true,
            "--help" | "-h" => result.help = true,
            _ => return Err(format!("unknown or duplicate desktop option: {argument}")),
        }
    }
    Ok(result)
}

pub fn load_configuration(
    explicit: Option<&Path>,
    app_config_directory: &Path,
) -> Result<LoadedConfiguration, String> {
    let path = match explicit {
        Some(path) => path.to_owned(),
        None => {
            fs::create_dir_all(app_config_directory).map_err(|error| {
                format!("cannot create desktop configuration directory: {error}")
            })?;
            let path = app_config_directory.join("desktop.toml");
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => file
                    .write_all(DEFAULT_CONFIG.as_bytes())
                    .map_err(|error| format!("cannot initialize desktop configuration: {error}"))?,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(format!("cannot initialize desktop configuration: {error}"));
                }
            }
            path
        }
    };
    let path = path.canonicalize().map_err(|error| {
        format!(
            "cannot locate desktop configuration {}: {error}",
            path.display()
        )
    })?;
    let text = fs::read_to_string(&path).map_err(|error| {
        format!(
            "cannot read desktop configuration {}: {error}",
            path.display()
        )
    })?;
    let mut config = ClientConfig::from_toml(&text)
        .map_err(|error| format!("invalid desktop configuration {}: {error}", path.display()))?;
    config.resolve_paths(&path);
    let mut server_url = url::Url::parse(&config.server_url).map_err(|error| error.to_string())?;
    server_url
        .set_scheme("http")
        .map_err(|()| "invalid desktop server origin")?;
    Ok(LoadedConfiguration {
        config,
        config_path: path,
        server_url: server_url.as_str().trim_end_matches('/').to_owned(),
    })
}
