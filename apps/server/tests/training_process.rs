//! 真实服务器进程的训练 HTTP、模型试听持久化及 SIGTERM 子树清理回归。
#![cfg(target_os = "linux")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Fixture {
    root: PathBuf,
    config: PathBuf,
    base: String,
    tts_port: u16,
    child: Option<Child>,
    client: reqwest::Client,
}
impl Fixture {
    async fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .canonicalize()
            .unwrap()
            .join(format!("server-training-process-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("engine/GPT_SoVITS")).unwrap();
        fs::write(
            root.join("engine/GPT_SoVITS/s2_train.py"),
            "# controlled engine fixture\n",
        )
        .unwrap();
        executable(
            &root.join("bin/nvidia-smi"),
            "#!/usr/bin/python3\nprint('Test GPU, 6141, 0')\n",
        );
        executable(
            &root.join("bin/fake-python"),
            r#"#!/usr/bin/python3
import sys,json,os,time,subprocess,hashlib
from pathlib import Path
if '--audio' in sys.argv:
    audio=Path(sys.argv[sys.argv.index('--audio')+1])
    assert audio.is_file() and audio.parent.name.startswith('.transcribe-')
    assert audio.parent.parent.name=='training'
    language=sys.argv[sys.argv.index('--language')+1]
    if language=='en': sys.exit(1)
    print(json.dumps({'text':'已自动识别，可继续校对。','language':language}),flush=True)
    sys.exit(0)
jobfile=Path(sys.argv[sys.argv.index('--job')+1])
job=json.loads(jobfile.read_text())
root=jobfile.parent
assert Path.cwd()==root
assert os.environ['MEOWLIVE_GPT_SOVITS_ROOT']
assert len(job['clips'])==2 and job['fp16'] is True
if job['name']=='audio-only':
    assert job['text_mode']=='audio_only' and all(c['text']=='' for c in job['clips'])
    (root/'automatic-transcription-requested.json').write_text(json.dumps(job['clips']))
else:
    assert job['text_mode']=='reviewed_text' and all(c['text'] for c in job['clips'])
if job['name'].startswith('wait'):
    child=subprocess.Popen(['/usr/bin/python3','-c','import time;time.sleep(60)'])
    (root/'owned-pids.json').write_text(json.dumps([os.getpid(),child.pid]))
    print(json.dumps({'state':'training','progress':40}),flush=True)
    time.sleep(60)
else:
    print(json.dumps({'state':'training','progress':50}),flush=True)
    hashes={}
    for name,path in job['artifacts'].items():
        if job['name']=='partial-pair' and name=='sovits': continue
        data=('controlled '+name+' weights').encode()
        (root/path).write_bytes(data)
        hashes[name]=hashlib.sha256(data).hexdigest()
    (root/'artifacts/manifest.json').write_text(json.dumps({'job_id':job['id'],'model_version':'v2','sha256':hashes}))
    print(json.dumps({'state':'succeeded','progress':100}),flush=True)
"#,
        );
        let server_port = unused_port();
        let tts_port = unused_port();
        let config = root.join("server.toml");
        let quote = |path: PathBuf| serde_json::to_string(&path.to_string_lossy()).unwrap();
        fs::write(
            &config,
            format!(
                r#"
[server]
listen_address="127.0.0.1:{server_port}"
[speech]
base_url="http://127.0.0.1:{tts_port}"
timeout_seconds=5
[resources]
directory={resources}
[training]
enabled=true
directory={training}
python={python}
engine_root={engine}
timeout_seconds=60
managed_inference=true
default_gpt_weights={default_gpt}
default_sovits_weights={default_sovits}
"#,
                resources = quote(root.join("resources")),
                training = quote(root.join("training")),
                python = quote(root.join("bin/fake-python")),
                engine = quote(root.join("engine")),
                default_gpt = quote(root.join("default.ckpt")),
                default_sovits = quote(root.join("default.pth"))
            ),
        )
        .unwrap();
        let mut fixture = Self {
            root,
            config,
            base: format!("http://127.0.0.1:{server_port}"),
            tts_port,
            child: None,
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        };
        fixture.start().await;
        fixture
    }
    async fn start(&mut self) {
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("server.log"))
            .unwrap();
        self.child = Some(
            Command::new(env!("CARGO_BIN_EXE_meowlive-server"))
                .args(["--config", self.config.to_str().unwrap()])
                .env(
                    "PATH",
                    format!("{}:/usr/bin:/bin", self.root.join("bin").display()),
                )
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if self
                .client
                .get(format!("{}/api/training", self.base))
                .send()
                .await
                .is_ok()
            {
                break;
            }
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "server exited: {}",
                fs::read_to_string(self.root.join("server.log")).unwrap()
            );
            assert!(Instant::now() < deadline, "server startup timed out");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    async fn terminate(&mut self) {
        let child = self.child.as_mut().unwrap();
        signal(child.id(), 15);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "SIGTERM must exit gracefully: {status}");
                break;
            }
            assert!(Instant::now() < deadline, "SIGTERM cleanup timed out");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        self.child = None;
    }
    async fn snapshot(&self) -> Value {
        self.client
            .get(format!("{}/api/training", self.base))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }
    async fn json(&self, path: &str, value: Value) -> (u16, Value) {
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .json(&value)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        (status, response.json().await.unwrap())
    }
    async fn upload(&self, path: &str, metadata: Value, count: usize) -> (u16, Value) {
        let boundary = "meowlive-process-boundary";
        let mut body=format!("--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"\r\nContent-Type: application/json\r\n\r\n{metadata}\r\n").into_bytes();
        for index in 0..count {
            body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"{index}.wav\"\r\nContent-Type: audio/wav\r\n\r\n").as_bytes());
            body.extend(wav());
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body)
            .send()
            .await
            .unwrap();
        (response.status().as_u16(), response.json().await.unwrap())
    }
    async fn voice(&self) -> String {
        let (status, body) = self
            .upload(
                "/api/voices",
                json!({"name":"测试音色","language":"zh","reference_text":"已经核对的参考文本"}),
                1,
            )
            .await;
        assert_eq!(status, 200, "{body}");
        body["voices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] != "default")
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    async fn train(&self, voice: &str, name: &str) -> String {
        let (status,body)=self.upload("/api/training/jobs",json!({"name":name,"voice_id":voice,"sovits_epochs":1,"gpt_epochs":1,"reviewed":true,"clips":[{"text":"第一个已核对片段","language":"zh"},{"text":"第二个已核对片段","language":"zh"}]}),2).await;
        assert_eq!(status, 202, "{body}");
        body["id"].as_str().unwrap().into()
    }
    async fn wait_state(&self, id: &str, wanted: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let snapshot = self.snapshot().await;
            let job = snapshot["jobs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|j| j["id"] == id)
                .unwrap();
            if job["status"] == wanted {
                return snapshot;
            }
            assert!(Instant::now() < deadline, "wanted {wanted}: {snapshot}");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    async fn owned_pids(&self, id: &str) -> Vec<u32> {
        let path = self
            .root
            .join("training/jobs")
            .join(id)
            .join("owned-pids.json");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(bytes) = fs::read(&path) {
                if let Ok(pids) = serde_json::from_slice(&bytes) {
                    return pids;
                }
            }
            assert!(Instant::now() < deadline, "runner did not start");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Ok(entries) = fs::read_dir(self.root.join("training/jobs")) {
            for entry in entries.flatten() {
                if let Ok(bytes) = fs::read(entry.path().join("owned-pids.json")) {
                    if let Ok(pids) = serde_json::from_slice::<Vec<u32>>(&bytes) {
                        for pid in pids {
                            if running(pid) {
                                signal(pid, 9);
                            }
                        }
                    }
                }
            }
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn executable(path: &PathBuf, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}
fn unused_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
fn signal(pid: u32, value: u8) {
    assert!(
        Command::new("/usr/bin/python3")
            .args([
                "-c",
                "import os,sys; os.kill(int(sys.argv[1]),int(sys.argv[2]))",
                &pid.to_string(),
                &value.to_string()
            ])
            .status()
            .unwrap()
            .success()
    );
}
fn running(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| {
            s.rsplit_once(") ")
                .map(|(_, tail)| !tail.starts_with("Z ") && !tail.starts_with("X "))
        })
        .unwrap_or(false)
}
fn wav() -> Vec<u8> {
    let data_size = 48000u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data_size).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data_size.to_le_bytes());
    for _ in 0..24000 {
        bytes.extend(1000i16.to_le_bytes());
    }
    bytes
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_binary_training_audition_save_and_activation_persist_across_restart() {
    let mut f = Fixture::new().await;
    let voice = f.voice().await;
    let id = f.train(&voice, "complete-pair").await;
    let complete = f.wait_state(&id, "completed").await;
    assert_eq!(complete["versions"][0]["available"], true);
    assert_eq!(complete["versions"][0]["saved"], false);
    assert_eq!(f.json("/api/training/save", json!({"id":id})).await.0, 409);
    assert_eq!(
        f.json("/api/training/save", json!({"id":"missing"}))
            .await
            .0,
        404
    );
    assert_eq!(f.json("/api/training/save", json!({})).await.0, 400);
    assert_eq!(
        f.json("/api/training/activate", json!({"id":id})).await.0,
        409
    );
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    async fn weights(
        State(calls): State<Arc<Mutex<Vec<String>>>>,
        Query(q): Query<HashMap<String, String>>,
    ) -> Json<Value> {
        calls.lock().unwrap().push(q["weights_path"].clone());
        Json(json!({"message":"success"}))
    }
    async fn tts(
        State(calls): State<Arc<Mutex<Vec<String>>>>,
        Json(body): Json<Value>,
    ) -> ([(axum::http::HeaderName, &'static str); 1], Vec<u8>) {
        assert!(body["ref_audio_path"].as_str().unwrap().ends_with(".wav"));
        calls.lock().unwrap().push("tts".into());
        ([(axum::http::header::CONTENT_TYPE, "audio/wav")], wav())
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", f.tts_port))
        .await
        .unwrap();
    let app = Router::new()
        .route("/set_gpt_weights", get(weights))
        .route("/set_sovits_weights", get(weights))
        .route("/tts", post(tts))
        .with_state(calls.clone());
    let fake = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let audio = f
        .client
        .post(format!("{}/api/training/audition", f.base))
        .json(&json!({"version_id":id,"text":"你好，欢迎来到直播间。"}))
        .send()
        .await
        .unwrap();
    assert_eq!(audio.status(), 200, "{}", audio.text().await.unwrap());
    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            f.root
                .join("training/jobs")
                .join(&id)
                .join("artifacts/gpt.ckpt")
                .to_string_lossy()
                .into_owned(),
            f.root
                .join("training/jobs")
                .join(&id)
                .join("artifacts/sovits.pth")
                .to_string_lossy()
                .into_owned(),
            "tts".into()
        ]
    );
    f.terminate().await;
    let config = fs::read_to_string(&f.config).unwrap();
    fs::write(
        &f.config,
        config.replace("managed_inference=true", "managed_inference=false"),
    )
    .unwrap();
    f.start().await;
    let (status, saved) = f.json("/api/training/save", json!({"id":id})).await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["versions"][0]["saved"], true);
    assert_eq!(saved["versions"][0]["active"], false);
    f.terminate().await;
    fs::write(&f.config, config).unwrap();
    f.start().await;
    assert_eq!(f.snapshot().await["versions"][0]["saved"], true);
    let (status, active) = f.json("/api/training/activate", json!({"id":id})).await;
    assert_eq!(status, 200, "{active}");
    assert_eq!(active["versions"][0]["active"], true);
    f.terminate().await;
    f.start().await;
    let snapshot = f.snapshot().await;
    assert_eq!(snapshot["versions"][0]["id"], id);
    assert_eq!(snapshot["versions"][0]["active"], true);
    assert_eq!(snapshot["versions"][0]["auditioned"], true);
    assert_eq!(snapshot["versions"][0]["saved"], true);
    f.terminate().await;
    fake.abort();
    let _ = fake.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_binary_http_cancel_and_sigterm_reap_owned_training_tree() {
    let mut f = Fixture::new().await;
    let voice = f.voice().await;
    let cancelled = f.train(&voice, "wait-cancel").await;
    let pids = f.owned_pids(&cancelled).await;
    assert_eq!(f.snapshot().await["busy"], true);
    let (code, body) = f
        .json("/api/training/cancel", json!({"id":cancelled}))
        .await;
    assert_eq!(code, 200, "{body}");
    let terminal = f.wait_state(&cancelled, "cancelled").await;
    assert_eq!(terminal["busy"], false);
    assert!(
        pids.iter().all(|pid| !running(*pid)),
        "cancel left owned processes alive"
    );
    let stopped = f.train(&voice, "wait-sigterm").await;
    let pids = f.owned_pids(&stopped).await;
    f.terminate().await;
    assert!(
        pids.iter().all(|pid| !running(*pid)),
        "SIGTERM left owned training tree alive"
    );
    f.start().await;
    let terminal = f.wait_state(&stopped, "cancelled").await;
    assert_eq!(terminal["busy"], false);
    assert_eq!(terminal["versions"], json!([]));
    f.terminate().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_binary_rejects_partial_pair_and_recovers_crashed_job_as_interrupted() {
    let mut f = Fixture::new().await;
    let voice = f.voice().await;
    let incomplete = f.train(&voice, "partial-pair").await;
    let failed = f.wait_state(&incomplete, "failed").await;
    assert_eq!(failed["busy"], false);
    assert_eq!(failed["versions"], json!([]));
    assert_eq!(
        f.json("/api/training/activate", json!({"id": incomplete}))
            .await
            .0,
        400
    );
    let interrupted = f.train(&voice, "wait-crash").await;
    let pids = f.owned_pids(&interrupted).await;
    // SIGKILL cannot run graceful cleanup. The fixture explicitly terminates only
    // its own recorded fake tree before checking persisted restart semantics.
    let mut child = f.child.take().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    for pid in pids {
        if running(pid) {
            signal(pid, 9);
        }
    }
    f.start().await;
    let recovered = f.wait_state(&interrupted, "interrupted").await;
    assert_eq!(recovered["busy"], false);
    assert_eq!(recovered["versions"], json!([]));
    let next = f.train(&voice, "complete-after-restart").await;
    f.wait_state(&next, "completed").await;
    f.terminate().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_binary_audio_only_upload_reaches_runner_and_transcription_preview_is_bounded() {
    let mut f = Fixture::new().await;
    let voice = f.voice().await;
    let input = json!({"name":"audio-only", "voice_id":voice, "sovits_epochs":1, "gpt_epochs":1,
        "reviewed":false, "text_mode":"audio_only", "clips":[{"language":"zh"},{"language":"zh"}]});
    for bad in [
        {
            let mut value = input.clone();
            value["text_mode"] = "reviewed_text".into();
            value["reviewed"] = true.into();
            value
        },
        {
            let mut value = input.clone();
            value["clips"][0]["text"] = "unexpected manual text".into();
            value
        },
        {
            let mut value = input.clone();
            value.as_object_mut().unwrap().remove("text_mode");
            value
        },
    ] {
        let (code, body) = f.upload("/api/training/jobs", bad, 2).await;
        assert_eq!(code, 400, "{body}");
    }
    assert!(f.snapshot().await["jobs"].as_array().unwrap().is_empty());
    let (code, body) = f.upload("/api/training/jobs", input, 2).await;
    assert_eq!(code, 202, "{body}");
    let id = body["id"].as_str().unwrap();
    f.wait_state(id, "completed").await;
    let directory = f.root.join("training/jobs").join(id);
    let manifest: Value =
        serde_json::from_slice(&fs::read(directory.join("job.json")).unwrap()).unwrap();
    assert_eq!(manifest["text_mode"], "audio_only");
    assert_eq!(manifest["clips"][0]["text"], "");
    assert_eq!(manifest["clips"][1]["text"], "");
    assert!(
        directory
            .join("automatic-transcription-requested.json")
            .exists()
    );
    let (code, body) = f
        .upload("/api/training/transcribe", json!({"language":"zh"}), 1)
        .await;
    assert_eq!(code, 200, "{body}");
    assert_eq!(
        body,
        json!({"text":"已自动识别，可继续校对。", "language":"zh"})
    );
    let (code, body) = f
        .upload("/api/training/transcribe", json!({"language":"en"}), 1)
        .await;
    assert_eq!(code, 500, "{body}");
    assert!(body["message"].as_str().unwrap().contains("手动填写"));
    assert!(!f.snapshot().await["busy"].as_bool().unwrap());
    assert!(fs::read_dir(f.root.join("training")).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".transcribe-")
    }));
    f.terminate().await;
}
