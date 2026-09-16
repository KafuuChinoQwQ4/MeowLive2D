//! 仅本地保存的 VTS token；不记录内容，不读取符号链接，不覆盖已有文件。
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct StoredToken {
    authentication_token: String,
}

pub(super) fn validate(token: &str) -> Result<(), String> {
    if token.is_empty() || token.len() > 64 || !token.is_ascii() {
        return Err("VTS 本地授权令牌格式无效".into());
    }
    Ok(())
}

fn check_ancestors(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("VTS 授权文件路径不能包含符号链接".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("无法检查 VTS 本地授权文件".into()),
        }
    }
    Ok(())
}

fn load_file(path: &Path) -> Result<Option<String>, String> {
    check_ancestors(path)?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("无法读取 VTS 本地授权文件".into()),
    };
    if !metadata.is_file() || metadata.len() > 4096 {
        return Err("VTS 本地授权文件格式无效".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("VTS 本地授权文件须仅允许当前用户访问".into());
        }
    }
    let file = fs::File::open(path).map_err(|_| "无法读取 VTS 本地授权文件")?;
    // Recheck the path after opening, before reading any secret bytes.
    check_ancestors(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let opened = file.metadata().map_err(|_| "无法检查 VTS 本地授权文件")?;
        let current = fs::symlink_metadata(path).map_err(|_| "无法检查 VTS 本地授权文件")?;
        if opened.ino() != current.ino() || opened.dev() != current.dev() {
            return Err("VTS 本地授权文件已变化".into());
        }
    }
    let mut bytes = Vec::new();
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取 VTS 本地授权文件")?;
    if bytes.len() > 4096 {
        return Err("VTS 本地授权文件超过大小限制".into());
    }
    let stored: StoredToken =
        serde_json::from_slice(&bytes).map_err(|_| "VTS 本地授权文件格式无效")?;
    validate(&stored.authentication_token)?;
    Ok(Some(stored.authentication_token))
}

fn save_file(path: &Path, token: &str) -> Result<(), String> {
    validate(token)?;
    check_ancestors(path)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|_| "无法创建 VTS 本地授权目录")?;
    }
    check_ancestors(path)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "无法创建 VTS 本地授权文件（不会覆盖已有文件）")?;
    let bytes = serde_json::to_vec(&StoredToken {
        authentication_token: token.into(),
    })
    .map_err(|_| "无法编码 VTS 本地授权文件")?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "无法保存 VTS 本地授权文件")
        .map_err(String::from)
}

// Dropping an awaited file operation cancels it if still queued. An OS write
// already in progress cannot be interrupted: it finishes the authenticated
// token save in its private file, without blocking the audio executor.
struct Blocking<T>(tokio::task::JoinHandle<Result<T, String>>);
impl<T> Drop for Blocking<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let mut task = Blocking(tokio::task::spawn_blocking(operation));
    (&mut task.0)
        .await
        .map_err(|_| "VTS 本地授权文件任务失败")?
}

pub(super) async fn load(path: PathBuf) -> Result<Option<String>, String> {
    blocking(move || load_file(&path)).await
}

pub(super) async fn save(path: PathBuf, token: String) -> Result<(), String> {
    blocking(move || save_file(&path, &token)).await
}
