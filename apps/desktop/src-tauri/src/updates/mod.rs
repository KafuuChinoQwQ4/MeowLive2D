//! Signed GitHub releases with content-addressed installer reconstruction.
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, VerifyingKey};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const REPOSITORY: &str = "KafuuChinoQwQ4/MeowLive2D";
const BLOCK_SIZE: u64 = 1_048_576;
const MAX_INSTALLER: u64 = 2 * 1024 * 1024 * 1024 - 1;

pub use meowlive_protocol::updates::UpdateStatus;
#[derive(Clone, Deserialize, Serialize)]
struct Envelope {
    payload: String,
    signature: String,
}
#[derive(Clone, Deserialize, Serialize)]
struct Chunk {
    sha256: String,
    size: u64,
}
#[derive(Clone, Deserialize, Serialize)]
struct Manifest {
    schema: u32,
    tag: String,
    #[serde(default)]
    target: Option<String>,
    installer: String,
    size: u64,
    sha256: String,
    chunks: Vec<Chunk>,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    assets: Vec<Asset>,
    body: Option<String>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
}
#[derive(Clone)]
struct Candidate {
    manifest: Manifest,
}

pub struct UpdateManager {
    cache_dir: PathBuf,
    status: Mutex<UpdateStatus>,
    candidate: Mutex<Option<Candidate>>,
    ready: Mutex<Option<(PathBuf, Manifest)>>,
    active: AtomicBool,
    cancelled: AtomicBool,
}
impl UpdateManager {
    pub fn new(cache_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            cache_dir,
            status: Mutex::new(UpdateStatus {
                phase: "idle".into(),
                current_tag: option_env!("MEOWLIVE_RELEASE_TAG")
                    .filter(|tag| !tag.is_empty())
                    .unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
                    .into(),
                available_tag: None,
                release_url: None,
                release_notes: None,
                message: "尚未检查更新".into(),
                downloaded_bytes: 0,
                reused_bytes: 0,
                total_bytes: 0,
            }),
            candidate: Mutex::new(None),
            ready: Mutex::new(None),
            active: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        })
    }
    pub fn status(&self) -> UpdateStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn is_busy(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
    fn change(&self, phase: &str, message: impl Into<String>) {
        let mut state = self.status.lock().unwrap_or_else(|e| e.into_inner());
        state.phase = phase.into();
        state.message = message.into();
    }
    fn begin(&self, phase: &str, message: &str) -> Result<(), String> {
        self.active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "已有更新操作正在运行".to_string())?;
        self.cancelled.store(false, Ordering::Release);
        self.change(phase, message);
        Ok(())
    }
    fn finish(&self, result: Result<(), String>) {
        let mut state = self.status.lock().unwrap_or_else(|e| e.into_inner());
        if self.cancelled.load(Ordering::Acquire) {
            *self.ready.lock().unwrap_or_else(|e| e.into_inner()) = None;
            state.phase = "cancelled".into();
            state.message = "更新已取消，可重新检查后重试".into();
        } else if let Err(error) = result {
            state.phase = "error".into();
            state.message = error;
        }
        self.active.store(false, Ordering::Release);
    }
    fn checkpoint(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Acquire) {
            Err("更新已取消，可重新检查后重试".into())
        } else {
            Ok(())
        }
    }
    pub fn cancel(&self) -> Result<(), String> {
        let mut state = self.status.lock().unwrap_or_else(|e| e.into_inner());
        if state.phase == "installing" {
            return Err("安装程序已启动，不能取消".into());
        }
        self.cancelled.store(true, Ordering::Release);
        if !self.is_busy() {
            *self.ready.lock().unwrap_or_else(|e| e.into_inner()) = None;
            state.phase = "cancelled".into();
            state.message = "更新已取消".into();
        } else {
            state.message = "正在取消更新，等待当前网络请求结束".into();
        }
        Ok(())
    }
    pub fn check(self: &Arc<Self>) -> Result<(), String> {
        self.begin("checking", "正在检查 GitHub Releases")?;
        *self.candidate.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.ready.lock().unwrap_or_else(|e| e.into_inner()) = None;
        {
            let mut state = self.status.lock().unwrap_or_else(|e| e.into_inner());
            state.available_tag = None;
            state.release_url = None;
            state.release_notes = None;
            state.total_bytes = 0;
            state.downloaded_bytes = 0;
            state.reused_bytes = 0;
        }
        let manager = Arc::clone(self);
        std::thread::spawn(move || {
            let result = manager.check_inner();
            manager.finish(result);
        });
        Ok(())
    }
    fn check_inner(&self) -> Result<(), String> {
        let client = client()?;
        let url = format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=100");
        let bytes = fetch(&client, &url, 32 * 1024 * 1024)?;
        let releases: Vec<Release> =
            serde_json::from_slice(&bytes).map_err(|_| "GitHub 发布列表格式无效")?;
        self.checkpoint()?;
        let current = version(&self.status().current_tag).ok_or("当前 App 发布标识无效")?;
        let latest = releases
            .into_iter()
            .filter(|release| !release.draft)
            .filter_map(|release| version(&release.tag_name).map(|v| (v, release)))
            .filter(|(v, _)| *v > current)
            .max_by_key(|(v, _)| *v);
        let Some((_, release)) = latest else {
            self.change("up_to_date", "当前已是最新版本");
            return Ok(());
        };
        {
            let mut state = self.status.lock().unwrap_or_else(|e| e.into_inner());
            state.available_tag = Some(release.tag_name.clone());
            state.release_url = Some(format!(
                "https://github.com/{REPOSITORY}/releases/tag/{}",
                release.tag_name
            ));
            state.release_notes = release.body.map(|body| body.chars().take(16000).collect());
        }
        let manifest_name = manifest_asset_name();
        if !release
            .assets
            .iter()
            .any(|asset| asset.name == manifest_name)
        {
            return Err(
                "发现新版本，但该发布缺少受签名的更新资源，请前往 GitHub Releases 查看".into(),
            );
        }
        let public_key = option_env!("MEOWLIVE_UPDATE_PUBLIC_KEY")
            .filter(|key| !key.is_empty())
            .ok_or("此 App 未配置更新签名公钥，已拒绝自动安装；请使用可信的正式安装包")?;
        let bytes = fetch(
            &client,
            &asset_url(&release.tag_name, manifest_name)?,
            2 * 1024 * 1024,
        )?;
        let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|_| "更新签名封装无效")?;
        let manifest = verify_manifest(&envelope, public_key, &release.tag_name)?;
        if !is_current_target(manifest.target.as_deref()) {
            return Err("该发布没有与当前安装包类型匹配的更新，已拒绝安装".into());
        }
        if !release
            .assets
            .iter()
            .any(|asset| asset.name == manifest.installer)
        {
            return Err("发布缺少完整安装包，无法安全回退".into());
        }
        self.checkpoint()?;
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .total_bytes = manifest.size;
        *self.candidate.lock().unwrap_or_else(|e| e.into_inner()) = Some(Candidate { manifest });
        self.change(
            "available",
            "发现可用更新；下载时将校验并复用本地分块，无缓存时完整下载",
        );
        Ok(())
    }
    pub fn prepare(self: &Arc<Self>) -> Result<(), String> {
        let candidate = self
            .candidate
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or("请先检查并验证可用更新")?;
        self.begin("downloading", "正在校验本地缓存并准备更新")?;
        *self.ready.lock().unwrap_or_else(|e| e.into_inner()) = None;
        {
            let mut status = self.status.lock().unwrap_or_else(|e| e.into_inner());
            status.downloaded_bytes = 0;
            status.reused_bytes = 0;
        }
        let manager = Arc::clone(self);
        std::thread::spawn(move || {
            let result = manager.prepare_inner(&candidate.manifest);
            manager.finish(result);
        });
        Ok(())
    }
    fn prepare_inner(&self, manifest: &Manifest) -> Result<(), String> {
        fs::create_dir_all(&self.cache_dir).map_err(io_error)?;
        let client = client()?;
        let partial = self.cache_dir.join("installer.partial");
        let result = (|| {
            let mut file = fs::File::create(&partial).map_err(io_error)?;
            let mut hasher = Sha256::new();
            let mut fallback = false;
            for chunk in &manifest.chunks {
                self.checkpoint()?;
                let cache = self.cache_dir.join(format!("{}.bin", chunk.sha256));
                let loaded = load_chunk(&cache, chunk, || {
                    let mut last_error = "更新分块下载失败".to_string();
                    for url in chunk_urls(&manifest.tag, manifest.target.as_deref(), &chunk.sha256)?
                    {
                        self.checkpoint()?;
                        match fetch(&client, &url, chunk.size) {
                            Ok(bytes) => return Ok(bytes),
                            Err(error) => last_error = error,
                        }
                    }
                    Err(last_error)
                });
                let bytes = match loaded {
                    Ok((bytes, reused)) => {
                        let mut state = self.status.lock().unwrap_or_else(|e| e.into_inner());
                        if reused {
                            state.reused_bytes += chunk.size;
                        } else {
                            state.downloaded_bytes += chunk.size;
                        }
                        bytes
                    }
                    Err(_) => {
                        fallback = true;
                        break;
                    }
                };
                hasher.update(&bytes);
                file.write_all(&bytes).map_err(io_error)?;
                let state = self.status();
                self.change(
                    "downloading",
                    if state.reused_bytes > 0 {
                        "正在增量下载：已复用经校验的本地分块"
                    } else {
                        "正在完整下载：尚无可复用分块"
                    },
                );
            }
            if fallback {
                drop(file);
                self.change("downloading", "分块资源不可用，正在回退到完整安装包下载");
                self.status
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .reused_bytes = 0;
                self.download_full(&client, manifest, &partial)?;
            } else {
                file.sync_all().map_err(io_error)?;
                if format!("{:x}", hasher.finalize()) != manifest.sha256 {
                    return Err("重建安装包的 SHA-256 校验失败".into());
                }
            }
            self.checkpoint()?;
            let destination = self
                .cache_dir
                .join(format!("installer-{}", manifest.sha256));
            if destination.exists() {
                fs::remove_file(&destination).map_err(io_error)?;
            }
            fs::rename(&partial, &destination).map_err(io_error)?;
            // Retain only this release's blocks as the next update's verified base.
            for entry in fs::read_dir(&self.cache_dir).map_err(io_error)?.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let keep = manifest
                    .chunks
                    .iter()
                    .any(|chunk| name == format!("{}.bin", chunk.sha256));
                if name.ends_with(".bin") && !keep {
                    let _ = fs::remove_file(entry.path());
                }
                if name.starts_with("installer-") && entry.path() != destination {
                    let _ = fs::remove_file(entry.path());
                }
            }
            *self.ready.lock().unwrap_or_else(|e| e.into_inner()) =
                Some((destination, manifest.clone()));
            self.change("ready", "安装包已通过签名与完整性校验，可以安装并重启 App");
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(partial);
        }
        result
    }
    fn download_full(
        &self,
        client: &Client,
        manifest: &Manifest,
        path: &std::path::Path,
    ) -> Result<(), String> {
        self.checkpoint()?;
        let mut response = client
            .get(asset_url(&manifest.tag, &manifest.installer)?)
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(network_error)?;
        let mut file = fs::File::create(path).map_err(io_error)?;
        let mut hasher = Sha256::new();
        for chunk in &manifest.chunks {
            self.checkpoint()?;
            let mut bytes = vec![0; chunk.size as usize];
            response.read_exact(&mut bytes).map_err(io_error)?;
            if !valid_chunk(&bytes, chunk) {
                return Err("完整安装包分块校验失败".into());
            }
            file.write_all(&bytes).map_err(io_error)?;
            hasher.update(&bytes);
            fs::write(self.cache_dir.join(format!("{}.bin", chunk.sha256)), &bytes)
                .map_err(io_error)?;
            self.status
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .downloaded_bytes += chunk.size;
        }
        let mut extra = [0; 1];
        if response.read(&mut extra).map_err(io_error)? != 0
            || format!("{:x}", hasher.finalize()) != manifest.sha256
        {
            return Err("完整安装包长度或 SHA-256 校验失败".into());
        }
        file.sync_all().map_err(io_error)?;
        Ok(())
    }
    pub fn launch_installer(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            self.launch_linux_update()
        }
        #[cfg(target_os = "windows")]
        {
            self.begin("installing", "正在启动已校验安装程序")?;
            let result: Result<(), String> = (|| {
                self.checkpoint()?;
                let (path, manifest) = self
                    .ready
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone()
                    .ok_or("尚未准备好更新")?;
                let _verified_handle = verify_file(&path, &manifest)?;
                std::process::Command::new(path)
                    .args(["/UPDATE", "/P", "/R"])
                    .spawn()
                    .map_err(io_error)?;
                Ok(())
            })();
            if let Err(error) = &result {
                self.change("error", error.clone());
            }
            self.active.store(false, Ordering::Release);
            result
        }
    }

    #[cfg(target_os = "linux")]
    fn launch_linux_update(&self) -> Result<(), String> {
        self.begin("installing", "正在应用已校验的 Linux 更新")?;
        let result = (|| {
            self.checkpoint()?;
            let (path, manifest) = self
                .ready
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
                .ok_or("尚未准备好更新")?;
            let _verified_handle = verify_file(&path, &manifest)?;
            match manifest.target.as_deref() {
                Some("linux-appimage-x86_64") => install_appimage(&path, &manifest),
                Some("linux-deb-x86_64") => install_deb(&path),
                _ => Err("更新包与当前 Linux 安装类型不匹配，已拒绝安装".into()),
            }
        })();
        if let Err(error) = &result {
            self.change("error", error.clone());
        }
        self.active.store(false, Ordering::Release);
        result
    }
}

fn manifest_asset_name() -> &'static str {
    #[cfg(windows)]
    {
        "meowlive-update.json"
    }
    #[cfg(target_os = "linux")]
    {
        match linux_package() {
            Some(LinuxPackage::AppImage) => "meowlive-update-linux-appimage.json",
            Some(LinuxPackage::Deb) => "meowlive-update-linux-deb.json",
            None => "meowlive-update-linux-unsupported.json",
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        "meowlive-update-unsupported.json"
    }
}

#[cfg(windows)]
fn is_current_target(target: Option<&str>) -> bool {
    matches!(target, Some("windows-x64") | None)
}
#[cfg(target_os = "linux")]
fn is_current_target(target: Option<&str>) -> bool {
    match (linux_package(), target) {
        (Some(LinuxPackage::AppImage), Some("linux-appimage-x86_64")) => true,
        (Some(LinuxPackage::Deb), Some("linux-deb-x86_64")) => true,
        _ => false,
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinuxPackage {
    AppImage,
    Deb,
}

#[cfg(target_os = "linux")]
fn linux_package() -> Option<LinuxPackage> {
    if std::env::consts::ARCH != "x86_64" {
        return None;
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return Some(LinuxPackage::AppImage);
    }
    let executable = std::env::current_exe().ok()?;
    if executable.starts_with("/usr/") || executable.starts_with("/opt/") {
        Some(LinuxPackage::Deb)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn install_appimage(download: &std::path::Path, manifest: &Manifest) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let installed = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("无法确定当前 AppImage 文件位置")?;
    let parent = installed.parent().ok_or("AppImage 安装目录无效")?;
    let staged = parent.join(format!(".meowlive-update-{}.AppImage", std::process::id()));
    let backup = parent.join(format!(".meowlive-backup-{}.AppImage", std::process::id()));
    let result = (|| {
        fs::copy(download, &staged).map_err(io_error)?;
        let permissions = fs::metadata(&installed).map_err(io_error)?.permissions();
        fs::set_permissions(
            &staged,
            fs::Permissions::from_mode(permissions.mode() | 0o111),
        )
        .map_err(io_error)?;
        fs::File::open(&staged)
            .and_then(|file| file.sync_all())
            .map_err(io_error)?;
        let _staged_verified_handle = verify_file(&staged, manifest)?;
        fs::rename(&installed, &backup).map_err(io_error)?;
        if let Err(error) = fs::rename(&staged, &installed) {
            let _ = fs::rename(&backup, &installed);
            return Err(io_error(error));
        }
        if let Err(error) = schedule_restart(&installed) {
            let _ = fs::remove_file(&installed);
            let _ = fs::rename(&backup, &installed);
            return Err(error);
        }
        let _ = fs::remove_file(backup);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(staged);
    }
    result
}

#[cfg(target_os = "linux")]
fn install_deb(deb: &std::path::Path) -> Result<(), String> {
    let status = std::process::Command::new("pkexec")
        .args(["dpkg", "--install"])
        .arg(deb)
        .status()
        .map_err(|error| format!("无法启动系统授权安装器（需要 pkexec 和 dpkg）：{error}"))?;
    if !status.success() {
        return Err("系统软件包安装未完成；更新包已保留，可重新尝试".into());
    }
    let executable = std::env::current_exe().map_err(io_error)?;
    schedule_restart(&executable)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn schedule_restart(executable: &std::path::Path) -> Result<(), String> {
    use std::process::Stdio;
    let pid = std::process::id().to_string();
    std::process::Command::new("sh")
        .args([
            "-c",
            "while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; exec \"$2\"",
            "meowlive-restart",
            &pid,
        ])
        .arg(executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(io_error)
}
fn io_error(error: std::io::Error) -> String {
    format!("更新文件操作失败：{error}")
}
fn network_error(error: reqwest::Error) -> String {
    format!("更新网络请求失败：{}", error.without_url())
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_chunk(bytes: &[u8], chunk: &Chunk) -> bool {
    bytes.len() as u64 == chunk.size && digest(bytes) == chunk.sha256
}
fn read_cached_chunk(path: &std::path::Path, chunk: &Chunk) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() != chunk.size {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(chunk.size + 1).read_to_end(&mut bytes).ok()?;
    valid_chunk(&bytes, chunk).then_some(bytes)
}
fn load_chunk(
    path: &std::path::Path,
    chunk: &Chunk,
    download: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<(Vec<u8>, bool), String> {
    if let Some(bytes) = read_cached_chunk(path, chunk) {
        return Ok((bytes, true));
    }
    let bytes = download()?;
    if !valid_chunk(&bytes, chunk) {
        return Err("下载分块校验失败".into());
    }
    fs::write(path, &bytes).map_err(io_error)?;
    Ok((bytes, false))
}
#[cfg(any(windows, target_os = "linux", test))]
fn verify_file(path: &std::path::Path, manifest: &Manifest) -> Result<fs::File, String> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Keep this handle alive through spawn: no writes, rename, or delete
        // may replace the executable between verification and CreateProcess.
        options.share_mode(1);
    }
    let mut file = options.open(path).map_err(io_error)?;
    if file.metadata().map_err(io_error)?.len() != manifest.size {
        return Err("安装包长度已变化".into());
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let read = file.read(&mut buffer).map_err(io_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if format!("{:x}", hasher.finalize()) != manifest.sha256 {
        return Err("安装前完整性复核失败".into());
    }
    Ok(file)
}
fn version(tag: &str) -> Option<(u64, u64, u64, u64)> {
    let text = tag.strip_prefix('v').unwrap_or(tag);
    let (base, preview) = if let Some((base, date)) = text.split_once("-windows-preview.") {
        if date.len() != 8 || !date.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        (base, date.parse().ok()?)
    } else {
        (text, u64::MAX)
    };
    let mut parts = base.split('.');
    let result = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        preview,
    );
    if parts.next().is_some() {
        None
    } else {
        Some(result)
    }
}
fn verify_manifest(envelope: &Envelope, public_key: &str, tag: &str) -> Result<Manifest, String> {
    let key = BASE64.decode(public_key).map_err(|_| "更新公钥编码无效")?;
    let key: [u8; 32] = key.try_into().map_err(|_| "更新公钥长度无效")?;
    let key = VerifyingKey::from_bytes(&key).map_err(|_| "更新公钥无效")?;
    let payload = BASE64
        .decode(&envelope.payload)
        .map_err(|_| "更新载荷编码无效")?;
    let signature = BASE64
        .decode(&envelope.signature)
        .map_err(|_| "更新签名编码无效")?;
    let signature = Signature::from_slice(&signature).map_err(|_| "更新签名长度无效")?;
    key.verify_strict(&payload, &signature)
        .map_err(|_| "更新签名验证失败，已拒绝安装")?;
    let manifest: Manifest = serde_json::from_slice(&payload).map_err(|_| "更新清单格式无效")?;
    let valid_hash = |text: &str| {
        text.len() == 64
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if manifest.schema != 1
        || manifest.tag != tag
        || version(tag).is_none()
        || manifest.size == 0
        || manifest.size > MAX_INSTALLER
        || !valid_hash(&manifest.sha256)
        || !matches!(
            manifest.target.as_deref(),
            Some("windows-x64") | Some("linux-appimage-x86_64") | Some("linux-deb-x86_64") | None
        )
        || !installer_matches_target(&manifest.installer, manifest.target.as_deref())
        || !manifest
            .installer
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_. -".contains(&b))
        || manifest.chunks.is_empty()
        || manifest.chunks.len() > 8192
    {
        return Err("更新清单内容不受支持".into());
    }
    let mut total = 0;
    for chunk in &manifest.chunks {
        if !valid_hash(&chunk.sha256) || chunk.size == 0 || chunk.size > BLOCK_SIZE {
            return Err("更新分块清单无效".into());
        }
        total += chunk.size;
    }
    if total != manifest.size {
        return Err("更新分块总长度不匹配".into());
    }
    Ok(manifest)
}
fn installer_matches_target(installer: &str, target: Option<&str>) -> bool {
    match target {
        Some("windows-x64") | None => installer.ends_with("-setup.exe"),
        Some("linux-appimage-x86_64") => installer.ends_with(".AppImage"),
        Some("linux-deb-x86_64") => installer.ends_with(".deb"),
        _ => false,
    }
}
fn chunk_urls(tag: &str, target: Option<&str>, hash: &str) -> Result<Vec<String>, String> {
    let name = format!("chunk-{hash}.bin");
    let update_tag = match target {
        Some("linux-appimage-x86_64") | Some("linux-deb-x86_64") => {
            format!("{tag}-updates-linux-x86_64")
        }
        _ => format!("{tag}-updates-windows-x86_64"),
    };
    Ok(vec![asset_url(&update_tag, &name)?, asset_url(tag, &name)?])
}
fn asset_url(tag: &str, name: &str) -> Result<String, String> {
    let mut url = url::Url::parse(&format!(
        "https://github.com/{REPOSITORY}/releases/download/"
    ))
    .map_err(|_| "更新源地址无效")?;
    url.path_segments_mut()
        .map_err(|_| "更新源路径无效")?
        .pop_if_empty()
        .push(tag)
        .push(name);
    Ok(url.into())
}
fn trusted_url(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "api.github.com"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent("MeowLive2D-Updater")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || !trusted_url(attempt.url()) {
                attempt.error("untrusted update redirect")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(network_error)
}
fn fetch(client: &Client, url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let parsed = url::Url::parse(url).map_err(|_| "更新地址无效")?;
    if !trusted_url(&parsed) {
        return Err("不受信任的更新来源".into());
    }
    let response = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(network_error)?;
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err("更新响应超出大小限制".into());
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > limit {
        return Err("更新响应超出大小限制".into());
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests;
