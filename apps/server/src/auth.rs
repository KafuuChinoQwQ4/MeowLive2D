//! 进程内管理员会话与独立设备凭据；只保留凭据摘要。
use crate::config::AuthConfig;
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    fmt,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};
use subtle::{Choice, ConstantTimeEq};

const MIN_CREDENTIAL_BYTES: usize = 32;
const MAX_CREDENTIAL_BYTES: usize = 512;
const MAX_SESSIONS: usize = 16;
const MAX_LOGIN_FAILURES: usize = 5;
const LOGIN_FAILURE_WINDOW: Duration = Duration::from_secs(60);

type TokenDigest = [u8; 32];

pub struct AdminAuth {
    mode: Mode,
    session_lifetime: Duration,
    sessions: Mutex<VecDeque<Session>>,
    login_failures: Mutex<VecDeque<Instant>>,
}

enum Mode {
    Disabled,
    Locked,
    Ready {
        admin: TokenDigest,
        device: TokenDigest,
    },
}

struct Session {
    token: TokenDigest,
    expires_at: Instant,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoginError {
    Disabled,
    Locked,
    Invalid,
    RateLimited,
}

pub struct NewSession {
    pub token: String,
    pub expires_in_seconds: u32,
}

impl fmt::Debug for AdminAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdminAuth")
            .field("enabled", &self.enabled())
            .field("configured", &self.configured())
            .field("session_lifetime", &self.session_lifetime)
            .finish_non_exhaustive()
    }
}

impl AdminAuth {
    pub fn disabled() -> Self {
        Self::new(Mode::Disabled, Duration::from_secs(8 * 60 * 60))
    }

    pub fn locked(session_lifetime_seconds: u32) -> Self {
        Self::new(
            Mode::Locked,
            Duration::from_secs(u64::from(session_lifetime_seconds)),
        )
    }

    pub fn from_config(config: &AuthConfig, config_path: &Path) -> Result<Self, String> {
        config.validate()?;
        if !config.enabled {
            return Ok(Self::disabled());
        }
        let admin = load_credential(
            "管理员",
            &config.admin_token_env,
            config.admin_token_file.as_deref(),
            config_path,
        )?;
        let device = load_credential(
            "设备",
            &config.device_token_env,
            config.device_token_file.as_deref(),
            config_path,
        )?;
        let admin_digest = digest(admin.as_bytes());
        let device_digest = digest(device.as_bytes());
        if bool::from(admin_digest.ct_eq(&device_digest)) {
            return Err("管理员与设备凭据必须相互独立".into());
        }
        Ok(Self::new(
            Mode::Ready {
                admin: admin_digest,
                device: device_digest,
            },
            Duration::from_secs(u64::from(config.session_lifetime_seconds)),
        ))
    }

    fn new(mode: Mode, session_lifetime: Duration) -> Self {
        Self {
            mode,
            session_lifetime,
            sessions: Mutex::new(VecDeque::with_capacity(MAX_SESSIONS)),
            login_failures: Mutex::new(VecDeque::with_capacity(MAX_LOGIN_FAILURES)),
        }
    }

    pub fn enabled(&self) -> bool {
        !matches!(self.mode, Mode::Disabled)
    }

    pub fn configured(&self) -> bool {
        matches!(self.mode, Mode::Ready { .. })
    }

    pub fn login(&self, credential: &str) -> Result<NewSession, LoginError> {
        let expected = match &self.mode {
            Mode::Disabled => return Err(LoginError::Disabled),
            Mode::Locked => return Err(LoginError::Locked),
            Mode::Ready { admin, .. } => admin,
        };
        let now = Instant::now();
        let mut failures = lock(&self.login_failures);
        prune_failures(&mut failures, now);
        if failures.len() >= MAX_LOGIN_FAILURES {
            return Err(LoginError::RateLimited);
        }
        if !constant_time_eq(expected, &digest(credential.as_bytes())) {
            failures.push_back(now);
            return Err(LoginError::Invalid);
        }
        failures.clear();
        drop(failures);

        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let mut sessions = lock(&self.sessions);
        prune_sessions(&mut sessions, now);
        if sessions.len() == MAX_SESSIONS {
            sessions.pop_front();
        }
        sessions.push_back(Session {
            token: digest(token.as_bytes()),
            expires_at: now + self.session_lifetime,
        });
        Ok(NewSession {
            token,
            expires_in_seconds: self.session_lifetime.as_secs() as u32,
        })
    }

    pub fn is_admin_session(&self, token: &str) -> bool {
        if !self.configured() {
            return false;
        }
        let now = Instant::now();
        let supplied = digest(token.as_bytes());
        let mut sessions = lock(&self.sessions);
        prune_sessions(&mut sessions, now);
        let mut matched = Choice::from(0);
        for session in sessions.iter() {
            matched |= session.token.ct_eq(&supplied);
        }
        bool::from(matched)
    }

    pub fn is_device(&self, token: &str) -> bool {
        match &self.mode {
            Mode::Ready { device, .. } => constant_time_eq(device, &digest(token.as_bytes())),
            Mode::Disabled | Mode::Locked => false,
        }
    }

    pub fn revoke(&self, token: &str) -> bool {
        let supplied = digest(token.as_bytes());
        let now = Instant::now();
        let mut sessions = lock(&self.sessions);
        prune_sessions(&mut sessions, now);
        let before = sessions.len();
        sessions.retain(|session| !constant_time_eq(&session.token, &supplied));
        sessions.len() != before
    }
}

fn load_credential(
    role: &str,
    env_name: &str,
    file: Option<&Path>,
    config_path: &Path,
) -> Result<String, String> {
    if !env_name.is_empty() {
        match std::env::var(env_name) {
            Ok(value) => return validate_credential(role, value),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(format!("{role}凭据环境变量编码无效"));
            }
            Err(std::env::VarError::NotPresent) => {}
        }
    }
    let path = file.ok_or_else(|| format!("{role}凭据未配置"))?;
    let path = resolve_path(path, config_path);
    let mut value = String::new();
    std::fs::File::open(&path)
        .map_err(|_| format!("无法读取{role}凭据文件"))?
        .take(1025)
        .read_to_string(&mut value)
        .map_err(|_| format!("无法读取{role}凭据文件"))?;
    validate_credential(role, value)
}

fn resolve_path(path: &Path, config_path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn validate_credential(role: &str, value: String) -> Result<String, String> {
    let value = value.trim().to_owned();
    if !(MIN_CREDENTIAL_BYTES..=MAX_CREDENTIAL_BYTES).contains(&value.len())
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(format!("{role}凭据必须为 32..512 字节 ASCII 可见字符"));
    }
    Ok(value)
}

fn digest(value: &[u8]) -> TokenDigest {
    Sha256::digest(value).into()
}

fn constant_time_eq(left: &TokenDigest, right: &TokenDigest) -> bool {
    bool::from(left.ct_eq(right))
}

fn prune_sessions(sessions: &mut VecDeque<Session>, now: Instant) {
    sessions.retain(|session| session.expires_at > now);
}

fn prune_failures(failures: &mut VecDeque<Instant>, now: Instant) {
    while failures
        .front()
        .is_some_and(|failure| now.duration_since(*failure) >= LOGIN_FAILURE_WINDOW)
    {
        failures.pop_front();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
