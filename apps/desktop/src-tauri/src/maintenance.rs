//! Native lifecycle operations: background IPC workers hold a single mutation lock.
#[cfg(windows)]
use crate::commands::DesktopState;
#[cfg(any(windows, test))]
use std::{io::Read, path::Path};

#[cfg(any(windows, test))]
fn busy(path: &str, data: &serde_json::Value) -> Result<bool, String> {
    Ok(match path {
        "/api/training" => data["busy"].as_bool().ok_or("训练状态无效")?,
        "/api/live" => !matches!(
            data["phase"].as_str(),
            Some("disabled" | "disconnected" | "failed")
        ),
        _ => data["speeches"]
            .as_array()
            .ok_or("播报状态无效")?
            .iter()
            .any(|s| {
                !matches!(
                    s["status"].as_str(),
                    Some("completed" | "failed" | "cancelled")
                )
            }),
    })
}

#[cfg(any(windows, test))]
fn credential(doc: &toml_edit::DocumentMut, path: &Path) -> Result<String, String> {
    let auth = doc.get("auth");
    let name = auth
        .and_then(|a| a.get("admin_token_env"))
        .and_then(|v| v.as_str())
        .unwrap_or("MEOWLIVE_ADMIN_TOKEN");
    let from_env = if name.is_empty() {
        None
    } else {
        match std::env::var(name) {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(_) => return Err("管理员凭据环境变量编码无效".into()),
        }
    };
    let value = if let Some(value) = from_env {
        value
    } else {
        let file = auth
            .and_then(|a| a.get("admin_token_file"))
            .and_then(|v| v.as_str())
            .ok_or("未找到本机主服务管理员凭据")?;
        let file = Path::new(file);
        let file = if file.is_absolute() {
            file.to_owned()
        } else {
            path.parent().ok_or("配置目录不可用")?.join(file)
        };
        let mut value = String::new();
        std::fs::File::open(file)
            .map_err(|_| "无法读取管理员凭据文件")?
            .take(1025)
            .read_to_string(&mut value)
            .map_err(|_| "无法读取管理员凭据文件")?;
        value
    };
    let value = value.trim();
    if !(32..=512).contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("管理员凭据格式无效".into());
    }
    Ok(value.into())
}

#[cfg(windows)]
fn json_response(response: reqwest::blocking::Response) -> Result<serde_json::Value, String> {
    let response = response
        .error_for_status()
        .map_err(|_| "无法确认主服务空闲或管理员认证失败")?;
    let mut bytes = Vec::new();
    response
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "读取主服务状态失败")?;
    if bytes.len() > 1024 * 1024 {
        return Err("主服务状态过大".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "主服务状态无效".into())
}

#[cfg(windows)]
pub fn require_idle(state: &DesktopState) -> Result<(), String> {
    let status = state
        .server
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .status();
    if !status.ready {
        return Err("主服务尚未就绪，无法确认任务已停止，请等待启动或处理服务错误".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let origin = state.server_url.trim_end_matches('/');
    let auth = json_response(
        client
            .get(format!("{origin}/api/admin/session"))
            .send()
            .map_err(|_| "无法查询主服务认证状态")?,
    )?;
    let session = if auth["enabled"].as_bool().ok_or("认证状态无效")? {
        let url = url::Url::parse(origin).map_err(|_| "主服务地址无效")?;
        if !status.managed
            || url.scheme() != "http"
            || !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        {
            return Err("外部认证主服务不支持由本 App 执行维护，请在其所属主机操作".into());
        }
        let path = Path::new(&state.config_path)
            .parent()
            .ok_or("配置目录不可用")?
            .join("server/server.toml");
        let doc = std::fs::read_to_string(&path)
            .map_err(|_| "无法读取主服务配置")?
            .parse::<toml_edit::DocumentMut>()
            .map_err(|_| "主服务配置无效")?;
        let token = credential(&doc, &path)?;
        let login = client
            .post(format!("{origin}/api/admin/session"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::json!({"token":token}).to_string())
            .send()
            .map_err(|_| "本机主服务认证失败")?;
        Some(
            json_response(login)?["token"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("管理员会话无效")?
                .to_owned(),
        )
    } else {
        None
    };
    let result = (|| {
        for path in ["/api/training", "/api/live", "/api/status"] {
            let mut request = client.get(format!("{origin}{path}"));
            if let Some(token) = &session {
                request = request.bearer_auth(token);
            }
            let data = json_response(request.send().map_err(|_| "无法查询主服务任务状态")?)?;
            if busy(path, &data)? {
                return Err("请先结束直播、训练和语音播报，再更改环境或安装更新".into());
            }
        }
        Ok(())
    })();
    if let Some(token) = session {
        let _ = client
            .delete(format!("{origin}/api/admin/session"))
            .bearer_auth(token)
            .send();
    }
    result
}

#[cfg(windows)]
pub fn restart_server(state: &DesktopState) -> Result<(), String> {
    start_server(state, true)
}

#[cfg(windows)]
pub fn start_server(state: &DesktopState, require_managed: bool) -> Result<(), String> {
    let directory = Path::new(&state.config_path)
        .parent()
        .ok_or("配置目录不可用")?;
    let environment = state.environment.snapshot();
    let distro = environment
        .selected_distro
        .filter(|name| {
            environment
                .distros
                .iter()
                .any(|item| item.name == *name && item.version == 2)
        })
        .ok_or("尚未检测到可用的 WSL2 发行版，请在环境与模型页完成检测")?;
    let mut server = state.server.lock().unwrap_or_else(|e| e.into_inner());
    server.shutdown();
    *server = crate::managed_server::ServerHandle::start_wsl(
        distro,
        directory.to_owned(),
        state.server_url.clone(),
    )?;
    drop(server);
    let result = wait_ready(
        || {
            state
                .server
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .status()
        },
        std::time::Duration::from_secs(185),
        require_managed,
    );
    if let Err(error) = &result {
        state
            .server
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown_failed(error.clone());
    }
    result
}

#[cfg(any(windows, test))]
fn wait_ready(
    mut status: impl FnMut() -> crate::managed_server::ServerStatus,
    timeout: std::time::Duration,
    require_managed: bool,
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let status = status();
        if let Some(error) = status.last_error {
            return Err(error);
        }
        if status.ready {
            return if status.managed || !require_managed {
                Ok(())
            } else {
                Err("重启时主服务端口被外部服务占用，未应用配置".into())
            };
        }
        if std::time::Instant::now() >= deadline {
            return Err("主服务重启超时".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[cfg(any(windows, test))]
fn replace_and_restart(
    path: &Path,
    original: &str,
    updated: &str,
    mut restart: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let backup = path.with_extension("toml.before-voice");
    std::fs::write(&backup, original).map_err(|_| "无法备份主服务配置")?;
    let temporary = path.with_extension("toml.part");
    std::fs::write(&temporary, updated).map_err(|_| "无法保存主服务配置")?;
    std::fs::remove_file(path).map_err(|_| "无法更新主服务配置")?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        std::fs::write(path, original).map_err(|restore| {
            format!("保存失败：{error}；恢复配置失败：{restore}；原配置备份已保留")
        })?;
        return Err(format!("保存失败，已恢复原配置：{error}"));
    }
    if let Err(error) = restart() {
        std::fs::write(path, original).map_err(|restore| {
            format!("重启失败：{error}；恢复配置失败：{restore}；原配置备份已保留")
        })?;
        restart().map_err(|restore| {
            format!("重启失败：{error}；原配置已恢复，但恢复后重启失败：{restore}")
        })?;
        return Err(format!("环境配置未应用，已恢复原配置和服务：{error}"));
    }
    Ok(())
}

#[cfg(windows)]
pub fn apply_environment(state: &DesktopState) -> Result<(), String> {
    let _guard = state.maintenance.lock().map_err(|_| "维护任务状态不可用")?;
    if state.updates.is_busy() {
        return Err("更新任务正在运行".into());
    }
    require_idle(state)?;
    let server = state
        .server
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .status();
    if !server.managed {
        return Err("当前连接外部主服务，请在其所属 App 配置训练环境".into());
    }
    let directory = Path::new(&state.config_path)
        .parent()
        .ok_or("配置目录不可用")?;
    let path = directory.join("server/server.toml");
    let original =
        std::fs::read_to_string(&path).map_err(|_| "主服务配置尚未建立，请等待主服务初始化")?;
    let snapshot = state.environment.snapshot();
    let updated = crate::environment_config::configure(&original, &snapshot)?;
    replace_and_restart(&path, &original, &updated, || restart_server(state))?;
    // A running process may have unloaded its models; let the backend verify readiness.
    state
        .environment
        .action(crate::environment::EnvironmentRequest {
            action: "start_inference".into(),
            distro: snapshot.selected_distro,
            model_id: None,
            config_path: None,
        })
        .map_err(|error| format!("主服务配置已应用，但推理启动未受理：{error}"))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_uses_protocol_status_and_rejects_unknown_states() {
        assert!(!busy("/api/status", &serde_json::json!({"speeches":[{"status":"completed"},{"status":"failed"},{"status":"cancelled"}]})).unwrap());
        assert!(
            busy(
                "/api/status",
                &serde_json::json!({"speeches":[{"status":"playing"}]})
            )
            .unwrap()
        );
        assert!(busy("/api/status", &serde_json::json!({"speeches":[{}]})).unwrap());
        assert!(busy("/api/live", &serde_json::json!({"phase":"connected"})).unwrap());
        assert!(busy("/api/training", &serde_json::json!({})).is_err());
    }
    #[test]
    fn restart_failure_restores_config_and_reports_recovery_failure() {
        let root = std::env::temp_dir().join(format!("maintenance-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("server.toml");
        std::fs::write(&path, "original").unwrap();
        let mut count = 0;
        let error = replace_and_restart(&path, "original", "updated", || {
            count += 1;
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                if count == 1 { "updated" } else { "original" }
            );
            Err("startup failed".into())
        })
        .unwrap_err();
        assert_eq!(count, 2);
        assert!(error.contains("恢复后重启失败"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn readiness_wait_observes_async_failure_and_requires_owned_server() {
        let mut calls = 0;
        let result = wait_ready(
            || {
                calls += 1;
                crate::managed_server::ServerStatus {
                    stopped: false,
                    ready: false,
                    managed: true,
                    last_error: if calls == 1 {
                        None
                    } else {
                        Some("config rejected".into())
                    },
                    log_path: String::new(),
                }
            },
            std::time::Duration::from_secs(1),
            true,
        );
        assert_eq!(result.unwrap_err(), "config rejected");
        assert!(calls >= 2);
        assert!(
            wait_ready(
                || crate::managed_server::ServerStatus {
                    stopped: false,
                    ready: true,
                    managed: false,
                    last_error: None,
                    log_path: String::new()
                },
                std::time::Duration::ZERO,
                true,
            )
            .is_err()
        );
    }
    #[test]
    fn explicit_start_can_reconnect_to_a_healthy_external_server() {
        assert!(
            wait_ready(
                || crate::managed_server::ServerStatus {
                    stopped: false,
                    ready: true,
                    managed: false,
                    last_error: None,
                    log_path: String::new(),
                },
                std::time::Duration::ZERO,
                false
            )
            .is_ok()
        );
    }
    #[test]
    fn admin_file_is_resolved_relative_to_server_configuration() {
        let root = std::env::temp_dir().join(format!("maintenance-auth-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("admin.secret"), "a".repeat(32)).unwrap();
        let doc: toml_edit::DocumentMut =
            "[auth]\nenabled=true\nadmin_token_env=''\nadmin_token_file='admin.secret'"
                .parse()
                .unwrap();
        assert_eq!(
            credential(&doc, &root.join("server.toml")).unwrap(),
            "a".repeat(32)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
