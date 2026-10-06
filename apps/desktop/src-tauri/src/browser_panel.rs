//! Authenticated loopback browser bridge for the packaged Windows desktop control panel.
use axum::Router;
use std::{path::Path, sync::Mutex, thread::JoinHandle};
use tokio::sync::oneshot;
use tower_http::services::{ServeDir, ServeFile};

#[cfg(any(windows, target_os = "linux"))]
use std::{
    net::{SocketAddr, TcpListener as StdTcpListener},
    thread,
};

#[cfg(any(windows, target_os = "linux"))]
const ADDRESS: &str = "127.0.0.1:1420";

type CommandFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>>;
type Backend = std::sync::Arc<dyn Fn(DesktopCommand) -> CommandFuture + Send + Sync>;

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(serde::Deserialize)]
#[serde(
    tag = "command",
    content = "args",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum DesktopCommand {
    DesktopStatus,
    DesktopServiceSetEnabled {
        id: String,
        enabled: bool,
    },
    EnvironmentStatus,
    EnvironmentAction {
        request: crate::environment::EnvironmentRequest,
    },
    EnvironmentApply,
    UpdateStatus,
    UpdateCheck,
    UpdatePrepare,
    UpdateCancel,
    UpdateInstall,
}

#[derive(Clone)]
struct BridgeState {
    backend: Backend,
    token: String,
}

fn json_response(
    status: axum::http::StatusCode,
    value: serde_json::Value,
) -> axum::response::Response {
    axum::response::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("cache-control", "no-store")
        .body(axum::body::Body::from(value.to_string()))
        .expect("valid JSON response")
}

fn error_response(status: axum::http::StatusCode, error: &str) -> axum::response::Response {
    json_response(status, serde_json::json!({"error": error}))
}

async fn validate_source(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::StatusCode;
    let headers = request.headers();
    let host = headers.get("host").and_then(|value| value.to_str().ok());
    if !matches!(host, Some("127.0.0.1:1420" | "localhost:1420")) {
        return error_response(StatusCode::FORBIDDEN, "不允许的控制面板主机");
    }
    if let Some(origin) = headers.get("origin") {
        if origin.to_str().ok() != Some(format!("http://{}", host.unwrap()).as_str()) {
            return error_response(StatusCode::FORBIDDEN, "不允许的控制面板来源");
        }
    }
    if headers
        .get("sec-fetch-site")
        .is_some_and(|value| value == "cross-site")
    {
        return error_response(StatusCode::FORBIDDEN, "不允许跨站访问控制面板");
    }
    next.run(request).await
}

async fn status(
    axum::extract::State(state): axum::extract::State<BridgeState>,
) -> axum::response::Response {
    match (state.backend)(DesktopCommand::DesktopStatus).await {
        Ok(status) => json_response(
            axum::http::StatusCode::OK,
            serde_json::json!({"status": status, "token": state.token}),
        ),
        Err(error) => error_response(axum::http::StatusCode::SERVICE_UNAVAILABLE, &error),
    }
}

async fn command(
    axum::extract::State(state): axum::extract::State<BridgeState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::http::StatusCode;
    if headers
        .get("x-meowlive-desktop-token")
        .and_then(|value| value.to_str().ok())
        != Some(state.token.as_str())
    {
        return error_response(StatusCode::FORBIDDEN, "控制面板令牌无效，请刷新页面");
    }
    if headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        != Some("application/json")
    {
        return error_response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "请使用 JSON 请求");
    }
    let command = match serde_json::from_slice::<DesktopCommand>(&body) {
        Ok(command) if !matches!(command, DesktopCommand::DesktopStatus) => command,
        _ => return error_response(StatusCode::BAD_REQUEST, "不支持的桌面命令或参数"),
    };
    match (state.backend)(command).await {
        Ok(result) => json_response(StatusCode::OK, serde_json::json!({"result": result})),
        Err(error) => error_response(StatusCode::CONFLICT, &error),
    }
}

fn router(root: &Path, backend: Backend, token: String) -> Router {
    Router::new()
        .route("/api/desktop/status", axum::routing::get(status))
        .route("/api/desktop/command", axum::routing::post(command))
        .route(
            "/api",
            axum::routing::any(|| async {
                error_response(axum::http::StatusCode::NOT_FOUND, "未知 API")
            }),
        )
        .route(
            "/api/",
            axum::routing::any(|| async {
                error_response(axum::http::StatusCode::NOT_FOUND, "未知 API")
            }),
        )
        .route(
            "/api/{*path}",
            axum::routing::any(|| async {
                error_response(axum::http::StatusCode::NOT_FOUND, "未知 API")
            }),
        )
        .fallback_service(
            ServeDir::new(root)
                .append_index_html_on_directories(true)
                .fallback(ServeFile::new(root.join("index.html"))),
        )
        .with_state(BridgeState { backend, token })
        .layer(axum::middleware::from_fn(validate_source))
}

#[cfg(windows)]
fn desktop_backend(app: tauri::AppHandle) -> Backend {
    std::sync::Arc::new(move |command| {
        let app = app.clone();
        Box::pin(async move {
            use crate::commands;
            use tauri::Manager;
            if app.try_state::<commands::DesktopState>().is_none() {
                return Err("桌面服务正在初始化，请稍后重试".into());
            }
            let result = match command {
                DesktopCommand::DesktopStatus => {
                    serde_json::to_value(commands::desktop_status(app).await?)
                }
                DesktopCommand::DesktopServiceSetEnabled { id, enabled } => serde_json::to_value(
                    commands::desktop_service_set_enabled(app, id, enabled).await?,
                ),
                DesktopCommand::EnvironmentStatus => {
                    serde_json::to_value(commands::environment_status(app.state()))
                }
                DesktopCommand::EnvironmentAction { request } => {
                    serde_json::to_value(commands::environment_action(app, request).await?)
                }
                DesktopCommand::EnvironmentApply => {
                    serde_json::to_value(commands::environment_apply(app).await?)
                }
                DesktopCommand::UpdateStatus => {
                    serde_json::to_value(commands::update_status(app.state()))
                }
                DesktopCommand::UpdateCheck => {
                    serde_json::to_value(commands::update_check(app.state())?)
                }
                DesktopCommand::UpdatePrepare => {
                    serde_json::to_value(commands::update_prepare(app.state())?)
                }
                DesktopCommand::UpdateCancel => {
                    serde_json::to_value(commands::update_cancel(app.state())?)
                }
                DesktopCommand::UpdateInstall => {
                    serde_json::to_value(commands::update_install(app).await?)
                }
            };
            result.map_err(|error| error.to_string())
        })
    })
}

#[cfg(target_os = "linux")]
fn desktop_backend(app: tauri::AppHandle) -> Backend {
    std::sync::Arc::new(move |command| {
        let app = app.clone();
        Box::pin(async move {
            use crate::platform::linux as commands;
            use tauri::Manager;
            if app.try_state::<commands::LinuxState>().is_none() {
                return Err("桌面服务正在初始化，请稍后重试".into());
            }
            let result = match command {
                DesktopCommand::DesktopStatus => {
                    serde_json::to_value(commands::desktop_status(app.state()))
                }
                DesktopCommand::DesktopServiceSetEnabled { id, enabled } => serde_json::to_value(
                    commands::desktop_service_set_enabled(app.state(), id, enabled)?,
                ),
                DesktopCommand::EnvironmentStatus => {
                    serde_json::to_value(commands::environment_status(app.state()))
                }
                DesktopCommand::EnvironmentAction { request } => {
                    serde_json::to_value(commands::environment_action(app.state(), request).await?)
                }
                DesktopCommand::EnvironmentApply => {
                    serde_json::to_value(commands::environment_apply()?)
                }
                DesktopCommand::UpdateStatus => {
                    serde_json::to_value(commands::update_status(app.state()))
                }
                DesktopCommand::UpdateCheck => {
                    serde_json::to_value(commands::update_check(app.state())?)
                }
                DesktopCommand::UpdatePrepare => {
                    serde_json::to_value(commands::update_prepare(app.state())?)
                }
                DesktopCommand::UpdateCancel => {
                    serde_json::to_value(commands::update_cancel(app.state())?)
                }
                DesktopCommand::UpdateInstall => {
                    serde_json::to_value(commands::update_install(app).await?)
                }
            };
            result.map_err(|error| error.to_string())
        })
    })
}

struct BrowserPanelInner {
    stop: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

/// Lives for the duration of the desktop app and serves only on loopback.
pub struct BrowserPanelHandle(Mutex<BrowserPanelInner>);

impl BrowserPanelHandle {
    #[cfg(any(windows, target_os = "linux"))]
    pub fn start(root: &Path, app: tauri::AppHandle) -> Result<Self, String> {
        if !root.join("index.html").is_file() {
            return Err(format!("浏览器控制面板文件不存在：{}", root.display()));
        }
        let address: SocketAddr = ADDRESS.parse().expect("valid loopback address");
        let listener = StdTcpListener::bind(address)
            .map_err(|error| format!("无法监听 {ADDRESS}，请检查端口是否被占用：{error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("无法初始化浏览器控制面板：{error}"))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("无法启动浏览器控制面板：{error}"))?;
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|error| format!("无法生成控制面板令牌：{error}"))?;
        let token = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let backend = desktop_backend(app);
        let (stop, stopped) = oneshot::channel();
        let root = root.to_owned();
        let worker = thread::Builder::new()
            .name("meowlive-browser-panel".into())
            .spawn(move || {
                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::from_std(listener) {
                        Ok(listener) => listener,
                        Err(error) => {
                            eprintln!("browser control panel listener failed: {error}");
                            return;
                        }
                    };
                    if let Err(error) = axum::serve(listener, router(&root, backend, token))
                        .with_graceful_shutdown(async {
                            let _ = stopped.await;
                        })
                        .await
                    {
                        eprintln!("browser control panel stopped: {error}");
                    }
                });
            })
            .map_err(|error| format!("无法启动浏览器控制面板线程：{error}"))?;
        Ok(Self(Mutex::new(BrowserPanelInner {
            stop: Some(stop),
            worker: Some(worker),
        })))
    }

    pub fn shutdown(&self) {
        let mut inner = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(stop) = inner.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = inner.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for BrowserPanelHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::{Backend, router};

    fn test_router(root: &std::path::Path) -> axum::Router {
        let backend: Backend = std::sync::Arc::new(|_| {
            Box::pin(async { Ok(serde_json::json!({"platform":"windows"})) })
        });
        router(root, backend, "test-token".into())
    }
    use axum::{body::to_bytes, http::StatusCode};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn browser_status_provides_session_token_and_prevents_cache() {
        let response = test_router(std::path::Path::new("."))
            .oneshot(
                axum::http::Request::get("/api/desktop/status")
                    .header("host", "127.0.0.1:1420")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"status":{"platform":"windows"},"token":"test-token"})
        );
    }

    #[tokio::test]
    async fn browser_commands_require_token_same_origin_and_allowlisted_payload() {
        for (token, origin, body, expected) in [
            (
                "",
                "http://127.0.0.1:1420",
                r#"{"command":"environment_status"}"#,
                StatusCode::FORBIDDEN,
            ),
            (
                "test-token",
                "https://attacker.example",
                r#"{"command":"environment_status"}"#,
                StatusCode::FORBIDDEN,
            ),
            (
                "test-token",
                "http://127.0.0.1:1420",
                r#"{"command":"arbitrary_invoke"}"#,
                StatusCode::BAD_REQUEST,
            ),
            (
                "test-token",
                "http://127.0.0.1:1420",
                r#"{"command":"desktop_service_set_enabled","args":{"id":"tts"}}"#,
                StatusCode::BAD_REQUEST,
            ),
            (
                "test-token",
                "http://127.0.0.1:1420",
                r#"{"command":"environment_status"}"#,
                StatusCode::OK,
            ),
            (
                "test-token",
                "http://127.0.0.1:1420",
                r#"{"command":"desktop_service_set_enabled","args":{"id":"tts","enabled":true}}"#,
                StatusCode::OK,
            ),
            (
                "test-token",
                "http://127.0.0.1:1420",
                r#"{"command":"environment_action","args":{"request":{"action":"detect"}}}"#,
                StatusCode::OK,
            ),
        ] {
            let response = test_router(std::path::Path::new("."))
                .oneshot(
                    axum::http::Request::post("/api/desktop/command")
                        .header("host", "127.0.0.1:1420")
                        .header("origin", origin)
                        .header("content-type", "application/json")
                        .header("x-meowlive-desktop-token", token)
                        .body(axum::body::Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{body}, {origin}, {token}");
        }
    }

    #[tokio::test]
    async fn browser_api_rejects_rebinding_hosts() {
        let response = test_router(std::path::Path::new("."))
            .oneshot(
                axum::http::Request::get("/api/desktop/status")
                    .header("host", "attacker.example:1420")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn browser_api_never_falls_back_to_spa() {
        let response = test_router(std::path::Path::new("."))
            .oneshot(
                axum::http::Request::get("/api/desktop/unknown")
                    .header("host", "127.0.0.1:1420")
                    .header("host", "127.0.0.1:1420")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn packaged_browser_router_serves_assets_and_spa_routes() {
        let path = std::env::temp_dir().join(format!(
            "meowlive-panel-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(path.join("assets")).unwrap();
        fs::write(path.join("index.html"), "<div id=\"root\">panel</div>").unwrap();
        fs::write(path.join("assets/app.js"), "window.panel=true").unwrap();
        let app = test_router(&path);

        let index = app
            .clone()
            .oneshot(
                axum::http::Request::get("/")
                    .header("host", "127.0.0.1:1420")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(index.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(index.into_body(), 1024).await.unwrap(),
            "<div id=\"root\">panel</div>"
        );
        let asset = app
            .clone()
            .oneshot(
                axum::http::Request::get("/assets/app.js")
                    .header("host", "127.0.0.1:1420")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(asset.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(asset.into_body(), 1024).await.unwrap(),
            "window.panel=true"
        );
        let unknown_api = app
            .clone()
            .oneshot(
                axum::http::Request::get("/api/desktop/unknown")
                    .header("host", "127.0.0.1:1420")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unknown_api.status(), StatusCode::NOT_FOUND);
        assert_eq!(unknown_api.headers()["content-type"], "application/json");
        let route = app
            .oneshot(
                axum::http::Request::get("/settings/voices")
                    .header("host", "127.0.0.1:1420")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(route.status(), StatusCode::OK);
        assert!(
            to_bytes(route.into_body(), 1024)
                .await
                .unwrap()
                .windows(5)
                .any(|value| value == b"panel")
        );
        fs::remove_dir_all(path).unwrap();
    }
}
