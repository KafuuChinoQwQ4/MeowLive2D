//! Translate host data paths for a selected WSL inference backend.
use meowlive_application::ports::speech::SynthesisError;
use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(super) async fn map(distribution: Option<&str>, path: &str) -> Result<String, SynthesisError> {
    let Some(distribution) = distribution else {
        return Ok(path.into());
    };
    if path.starts_with('/') {
        return Ok(path.into());
    }
    let path = normalize_windows_path(path);
    if let Some(path) = wsl_unc_path(distribution, &path)? {
        return Ok(path);
    }
    let distribution = distribution.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut child = Command::new("wsl.exe")
            .args([
                "--distribution",
                &distribution,
                "--exec",
                "wslpath",
                "-a",
                "-u",
                &path,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| failure())?;
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if start.elapsed() < Duration::from_secs(10) => {
                    thread::sleep(Duration::from_millis(20))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(failure());
                }
            }
        }
        let output = child.wait_with_output().map_err(|_| failure())?;
        let value = String::from_utf8(output.stdout).map_err(|_| failure())?;
        let value = value.trim();
        if !output.status.success() || !value.starts_with('/') || value.len() > 4096 {
            return Err(failure());
        }
        Ok(value.to_owned())
    })
    .await
    .map_err(|_| failure())?
}
fn failure() -> SynthesisError {
    SynthesisError::new("无法将语音资源路径映射到所选 WSL 环境")
}

fn normalize_windows_path(path: &str) -> String {
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{path}");
    }
    path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
}

fn wsl_unc_path(distribution: &str, path: &str) -> Result<Option<String>, SynthesisError> {
    let parts: Vec<_> = path.split('\\').collect();
    if parts.len() >= 4
        && parts[0].is_empty()
        && parts[1].is_empty()
        && (parts[2].eq_ignore_ascii_case("wsl.localhost") || parts[2].eq_ignore_ascii_case("wsl$"))
    {
        if !parts[3].eq_ignore_ascii_case(distribution) {
            return Err(failure());
        }
        return Ok(Some(format!("/{}", parts[4..].join("/"))));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_windows_paths_are_accepted_by_wslpath() {
        assert_eq!(
            normalize_windows_path(r"\\?\C:\voice data\a.wav"),
            r"C:\voice data\a.wav"
        );
        assert_eq!(
            normalize_windows_path(r"\\?\UNC\host\share\a.wav"),
            r"\\host\share\a.wav"
        );
    }
    #[test]
    fn wsl_unc_paths_are_scoped_to_selected_distribution() {
        let path = normalize_windows_path(r"\\?\UNC\wsl.localhost\Ubuntu\home\voice data\a.wav");
        assert_eq!(
            wsl_unc_path("Ubuntu", &path).unwrap(),
            Some("/home/voice data/a.wav".into())
        );
        assert!(wsl_unc_path("Other", &path).is_err());
        assert_eq!(
            wsl_unc_path("Ubuntu", r"\\wsl$\Ubuntu\opt\weights.ckpt").unwrap(),
            Some("/opt/weights.ckpt".into())
        );
    }
    #[tokio::test]
    async fn linux_paths_and_native_mode_need_no_wsl_process() {
        assert_eq!(
            map(Some("Ubuntu"), "/opt/model.ckpt").await.unwrap(),
            "/opt/model.ckpt"
        );
        assert_eq!(map(None, r"C:\audio.wav").await.unwrap(), r"C:\audio.wav");
    }
}
