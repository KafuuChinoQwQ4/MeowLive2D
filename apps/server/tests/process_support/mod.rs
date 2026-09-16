use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};

pub struct ServerProcess {
    child: Child,
    root: PathBuf,
    pub base: String,
}
impl ServerProcess {
    pub async fn start(config: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/agent-process-tests")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("server.toml");
        let configuration = if config.contains("[resources]") {
            config.to_owned()
        } else {
            format!(
                "{config}\n[resources]\ndirectory={}\n",
                serde_json::to_string(&root.join("resources")).unwrap()
            )
        };
        std::fs::write(&path, configuration).unwrap();
        let log = root.join("server.log");
        let child = Command::new(env!("CARGO_BIN_EXE_meowlive-server"))
            .args(["--config"])
            .arg(path)
            .env("MEOWLIVE_TEST_MODEL_KEY", "fake-process-test-key")
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&log).unwrap())
            .spawn()
            .unwrap();
        let mut process = Self {
            child,
            root,
            base: String::new(),
        };
        process.base = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let text = std::fs::read_to_string(&log).unwrap();
                if let Some(line) = text.lines().find(|line| line.contains("主服务：http://")) {
                    return format!("http://{}", line.split("http://").nth(1).unwrap());
                }
                assert!(
                    process.child.try_wait().unwrap().is_none(),
                    "server stopped before listening: {text}"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("server must start");
        process
    }
}
impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub fn wav() -> Vec<u8> {
    let mut bytes = Vec::new();
    let data_size = 4800u32;
    bytes.extend(b"RIFF");
    bytes.extend((36 + data_size).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(24000u32.to_le_bytes());
    bytes.extend(48000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data_size.to_le_bytes());
    for _ in 0..2400 {
        bytes.extend(1000i16.to_le_bytes());
    }
    bytes
}
