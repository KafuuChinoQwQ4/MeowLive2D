mod support;
use meowlive_server::transport::http::router;
use serde_json::json;

#[tokio::test]
async fn disabled_training_is_visible_and_never_accepts_a_version() {
    let state = support::state();
    let (code, snapshot) =
        support::request(router(state.clone()), "GET", "/api/training", json!({})).await;
    assert_eq!(code, 200);
    assert_eq!(snapshot["enabled"], false);
    assert_eq!(snapshot["versions"], json!([]));
    for path in [
        "/api/training/activate",
        "/api/training/cancel",
        "/api/training/save",
    ] {
        let (code, _) =
            support::request(router(state.clone()), "POST", path, json!({"id":"missing"})).await;
        assert_eq!(code, 409);
    }
}
#[tokio::test]
async fn cloud_preset_is_not_marked_as_verified_offline() {
    let state = support::state();
    let (code, snapshot) = support::request(
        router(state.clone()),
        "GET",
        "/api/runtime/preset",
        json!({}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(snapshot["mode"], "cloud");
    assert_eq!(snapshot["verified"], false);
    let (code, _) = support::request(
        router(state),
        "POST",
        "/api/runtime/measure",
        json!({"voice_id":"default","text":"你好"}),
    )
    .await;
    assert_eq!(code, 409);
}
