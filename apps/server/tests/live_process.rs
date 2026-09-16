#[allow(dead_code)]
mod process_support;
use process_support::ServerProcess;
use serde_json::Value;

#[tokio::test]
async fn executable_loads_live_credentials_but_does_not_connect_until_requested() {
    // The existing process fixture provides this fake key only to its child.
    // Reusing it for the three fields verifies wiring without changing global env.
    let process = ServerProcess::start(
        r#"
[server]
listen_address="127.0.0.1:0"
[live]
enabled=true
app_id=42
access_key_id_env="MEOWLIVE_TEST_MODEL_KEY"
access_key_secret_env="MEOWLIVE_TEST_MODEL_KEY"
identity_code_env="MEOWLIVE_TEST_MODEL_KEY"
"#,
    )
    .await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/live", process.base))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let snapshot: Value = response.json().await.unwrap();
    assert_eq!(snapshot["configured"], true);
    assert_eq!(snapshot["phase"], "disconnected");
    assert_eq!(snapshot["room_id"], Value::Null);
    assert_eq!(snapshot["accepted_events"], 0);
    assert_eq!(snapshot["reconnect_attempts"], 0);
    assert!(!snapshot.to_string().contains("fake-process-test-key"));
    assert!(!snapshot.to_string().contains("access_key"));
    let stopped: Value = client
        .post(format!("{}/api/live/disconnect", process.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stopped["phase"], "disconnected");
}
