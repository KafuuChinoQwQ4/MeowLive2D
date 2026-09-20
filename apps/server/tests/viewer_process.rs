#![allow(dead_code)]
mod process_support;
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use process_support::ServerProcess;
use serde_json::{Value, json};
use std::time::Duration;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error, client::IntoClientRequest, http::StatusCode},
};

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn default_owner_can_use_persistent_records_and_memory_without_login() {
    let database_url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let scope = format!("owner-{}", uuid::Uuid::new_v4());
    let receipts = std::env::temp_dir().join(&scope);
    // No auth settings or persistence opt-in: these are the product defaults.
    let config = format!(
        "[server]\nlisten_address='127.0.0.1:0'\n[viewers]\nscope_id='{scope}'\ndatabase_url_env='MEOWLIVE_TEST_DATABASE_URL'\nreceipt_directory={}\n",
        serde_json::to_string(&receipts).unwrap()
    );
    let mut server = ServerProcess::start(&config).await;
    let client = reqwest::Client::new();
    let session: Value = client
        .get(format!("{}/api/admin/session", server.base))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(session, json!({"enabled":false,"authenticated":true}));
    let event = json!({"events":[{"id":"owner-event","source":"simulator","viewer":"默认用户的观众","viewer_identity":{"namespace":"local","kind":"open_id","external_id":"owner-viewer"},"kind":{"type":"chat","text":"我喜欢猫"}}]});
    let outcome: Value = client
        .post(format!("{}/api/events", server.base))
        .json(&event)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(outcome["persisted"], 1);
    server.restart().await;
    let viewers: Value = client
        .get(format!("{}/api/admin/viewers", server.base))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(viewers["viewers"].as_array().unwrap().len(), 1);
    let viewer = viewers["viewers"][0]["viewer_id"].as_str().unwrap();
    for path in [
        "/api/admin/events".to_owned(),
        format!("/api/admin/viewers/{viewer}/memories"),
        format!("/api/admin/viewers/{viewer}/relationships"),
        "/api/admin/memories/status".to_owned(),
    ] {
        let response = client
            .get(format!("{}{path}", server.base))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200, "{path}");
    }
    // Simulator traffic is persisted but intentionally does not create a real
    // companionship ledger. The owner reaches this handler without a login.
    let detail = client
        .get(format!("{}/api/admin/viewers/{viewer}", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(detail.status(), 404);
    assert_eq!(
        detail.json::<Value>().await.unwrap()["code"],
        "viewer_not_found"
    );
    let duplicate: Value = client
        .post(format!("{}/api/events", server.base))
        .json(&event)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(duplicate["duplicates"], 1);
    drop(server);
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    for table in [
        "viewer_events",
        "viewer_aliases",
        "viewer_identities",
        "viewers",
        "live_sessions",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE scope_id=$1"))
            .bind(&scope)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
    let _ = std::fs::remove_dir_all(receipts);
}

async fn assert_device_websockets_reject(base: &str, token: &str) {
    for path in [
        "/ws/control",
        "/ws/audio?session_id=invalid&bridge_id=invalid",
    ] {
        let websocket_base = base.replacen("http://", "ws://", 1);
        let mut request = format!("{websocket_base}{path}")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("authorization", format!("Bearer {token}").parse().unwrap());
        let error = connect_async(request).await.unwrap_err();
        let Error::Http(response) = error else {
            panic!("expected HTTP authentication rejection, got {error}");
        };
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn authenticated_http_ingestion_survives_process_restart_without_replaying() {
    let database_url =
        std::env::var("MEOWLIVE_TEST_DATABASE_URL").expect("dedicated test database required");
    let root = std::env::temp_dir().join(format!("viewer-process-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let admin = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let device = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    std::fs::write(root.join("admin"), &admin).unwrap();
    std::fs::write(root.join("device"), &device).unwrap();
    let scope = format!("process-viewers-{}", uuid::Uuid::new_v4());
    let config = format!(
        "[server]\nlisten_address='127.0.0.1:0'\n[auth]\nenabled=true\nadmin_token_env=''\ndevice_token_env=''\nadmin_token_file={}\ndevice_token_file={}\n[viewers]\nenabled=true\nreceipt_directory={}\nscope_id='{scope}'\ndatabase_url_env='MEOWLIVE_TEST_DATABASE_URL'\n",
        serde_json::to_string(&root.join("admin")).unwrap(),
        serde_json::to_string(&root.join("device")).unwrap(),
        serde_json::to_string(&root.join("receipts")).unwrap()
    );
    let server = ServerProcess::start(&config).await;
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .get(format!("{}/api/admin/viewers", server.base))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let session: Value = client
        .post(format!("{}/api/admin/session", server.base))
        .json(&json!({"token":admin}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let session = session["token"].as_str().unwrap().to_owned();
    assert_device_websockets_reject(&server.base, &session).await;
    assert_device_websockets_reject(
        &server.base,
        "incorrect-device-credential-00000000000000000000000000000000",
    )
    .await;
    let desktop_config = ClientConfig {
        server_url: server.base.clone(),
        device_token_file: Some(root.join("device")),
        handshake_timeout_ms: 2_000,
        ..Default::default()
    };
    let desktop = tokio::spawn(async move {
        run_once(
            &desktop_config,
            SimulatedBackend::new(desktop_config.max_buffer_samples),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let status: Value = client
                .get(format!("{}/api/status", server.base))
                .bearer_auth(&session)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            if status["bridge_connected"] == true {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("device credential must connect and pair both WebSockets");
    let event = json!({"events":[{"id":"stable-event","source":"bilibili","viewer":"Original name","viewer_identity":{"namespace":"untrusted-source","kind":"open_id","external_id":"9007199254740993"},"kind":{"type":"chat","text":"hello"}}]});
    let outcome: Value = client
        .post(format!("{}/api/events", server.base))
        .bearer_auth(&session)
        .json(&event)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(outcome["persisted"], 1);
    let before: Value = client
        .get(format!("{}/api/admin/viewers", server.base))
        .bearer_auth(&session)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let viewer_id = before["viewers"][0]["viewer_id"].clone();
    assert_eq!(
        before["viewers"][0]["identities"][0]["external_id"],
        "9007199254740993"
    );
    assert_eq!(
        before["viewers"][0]["identities"][0]["platform"],
        "simulator"
    );
    drop(server);
    let desktop_result = tokio::time::timeout(Duration::from_secs(3), desktop)
        .await
        .expect("desktop must observe server shutdown")
        .unwrap();
    assert!(desktop_result.is_err());
    let server = ServerProcess::start(&config).await;
    assert_eq!(
        client
            .get(format!("{}/api/admin/viewers", server.base))
            .bearer_auth(&session)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let login: Value = client
        .post(format!("{}/api/admin/session", server.base))
        .json(&json!({"token":admin}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let session = login["token"].as_str().unwrap();
    let outcome: Value = client
        .post(format!("{}/api/events", server.base))
        .bearer_auth(session)
        .json(&event)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(outcome["duplicates"], 1);
    assert_eq!(outcome["persisted"], 0);
    let after: Value = client
        .get(format!("{}/api/admin/viewers", server.base))
        .bearer_auth(session)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(after["viewers"][0]["viewer_id"], viewer_id);
    let queued: Value = client
        .get(format!("{}/api/agent", server.base))
        .bearer_auth(session)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(queued["events"], json!([]));
    let history: Value = client
        .get(format!("{}/api/admin/events", server.base))
        .bearer_auth(session)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(history["events"].as_array().unwrap().len(), 1);
    assert!(history["events"][0]["occurred_at_ms"].as_u64().unwrap() > 1_700_000_000_000);
    assert!(
        history["events"][0]["session_id"]
            .as_str()
            .unwrap()
            .starts_with("simulator:")
    );
    drop(server);
    // Cleanup only this test's uniquely named scope, using the same restricted application account.
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    for table in [
        "viewer_events",
        "viewer_aliases",
        "viewer_identities",
        "viewers",
        "live_sessions",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE scope_id=$1"))
            .bind(&scope)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
