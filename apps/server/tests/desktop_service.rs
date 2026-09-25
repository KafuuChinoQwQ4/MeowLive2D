use meowlive_desktop::managed_server::ServerHandle;
use std::{
    net::TcpListener,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn fixture() -> (PathBuf, String) {
    let root =
        std::env::temp_dir().join(format!("meowlive-desktop-service-{}", uuid::Uuid::new_v4()));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    (root, url)
}

fn start(root: &std::path::Path, url: &str) -> ServerHandle {
    ServerHandle::start(
        env!("CARGO_BIN_EXE_meowlive-server").into(),
        root.into(),
        url.into(),
    )
    .unwrap()
}

fn ready(handle: &ServerHandle) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !handle.status().ready {
        assert!(Instant::now() < deadline, "{:?}", handle.status());
        assert!(
            handle.status().last_error.is_none(),
            "{:?}",
            handle.status()
        );
        thread::sleep(Duration::from_millis(30));
    }
}

#[test]
fn app_automatically_starts_real_server_and_shutdown_releases_port() {
    let (root, url) = fixture();
    let mut handle = start(&root, &url);
    ready(&handle);
    assert!(handle.status().managed);
    let path = root.join("server/server.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        !meowlive_server::config::AppConfig::parse(&text)
            .unwrap()
            .viewers
            .enabled
    );
    handle.shutdown();
    assert!(TcpListener::bind(url.trim_start_matches("http://")).is_ok());
    // Editing the persistent configuration must survive the next app launch.
    let edited = format!("{text}\n[agent]\ntopic='保留我的配置'\n");
    std::fs::write(&path, &edited).unwrap();
    let handle = start(&root, &url);
    ready(&handle);
    assert_eq!(std::fs::read_to_string(path).unwrap(), edited);
    drop(handle);
    assert!(TcpListener::bind(url.trim_start_matches("http://")).is_ok());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn app_reuses_external_server_without_terminating_it() {
    let (root, url) = fixture();
    let owner = start(&root, &url);
    ready(&owner);
    let observer = start(&root.join("other-app"), &url);
    ready(&observer);
    assert!(!observer.status().managed);
    drop(observer);
    assert!(owner.status().ready);
    assert!(std::net::TcpStream::connect(url.trim_start_matches("http://")).is_ok());
    drop(owner);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn occupied_port_is_visible_failure_and_never_launches_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let root = std::env::temp_dir().join(format!("meowlive-occupied-{}", uuid::Uuid::new_v4()));
    let handle = start(&root, &format!("http://{}", listener.local_addr().unwrap()));
    let deadline = Instant::now() + Duration::from_secs(5);
    while handle.status().last_error.is_none() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(30));
    }
    assert!(!handle.status().ready);
    assert!(!handle.status().managed);
    assert!(handle.status().last_error.unwrap().contains("占用"));
    assert!(!root.join("server/server.toml").exists());
    drop(handle);
    if root.exists() {
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn missing_bundled_executable_is_reported_without_hanging() {
    let (root, url) = fixture();
    let handle = ServerHandle::start(root.join("missing.exe"), root.clone(), url).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while handle.status().last_error.is_none() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(30));
    }
    assert!(!handle.status().ready);
    assert!(handle.status().last_error.unwrap().contains("启动主服务"));
    drop(handle);
    std::fs::remove_dir_all(root).unwrap();
}
