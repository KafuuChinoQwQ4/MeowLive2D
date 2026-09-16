mod avatar_support;
use avatar_support::*;
use meowlive_desktop_runtime::{avatar::AvatarStatus, presentation::AvatarDriver};
use serde_json::json;

#[tokio::test]
async fn changing_character_resets_old_parameter_before_acknowledging_new_input() {
    let fixture = Fixture::new(true).await;
    let driver = AvatarDriver::start(fixture.config.clone()).unwrap();
    let mut statuses = driver.status();
    let mut old = fixture.accept().await;
    authenticate(&mut old).await;
    assert_eq!(injected(&mut old).await, 0.0);
    status_is(&mut statuses, AvatarStatus::Connected).await;
    let control = driver.mouth_parameter();
    let selected = tokio::spawn(async move { control.select("WitchMouthOpen".into()).await });
    assert_eq!(injected(&mut old).await, 0.0);
    let mut new = fixture.accept().await;
    let req = request(&mut new, "AuthenticationRequest").await;
    respond(
        &mut new,
        &req,
        "AuthenticationResponse",
        json!({"authenticated":true}),
    )
    .await;
    let req = request(&mut new, "ParameterCreationRequest").await;
    assert_eq!(req["data"]["parameterName"], "WitchMouthOpen");
    assert!(
        !selected.is_finished(),
        "selection must wait for VTS parameter setup"
    );
    respond(
        &mut new,
        &req,
        "ParameterCreationResponse",
        json!({"parameterName":"WitchMouthOpen"}),
    )
    .await;
    let req = request(&mut new, "InjectParameterDataRequest").await;
    assert_eq!(req["data"]["parameterValues"][0]["id"], "WitchMouthOpen");
    assert_eq!(req["data"]["parameterValues"][0]["value"], 0.0);
    respond(&mut new, &req, "InjectParameterDataResponse", json!({})).await;
    assert!(selected.await.unwrap().is_ok());
    driver.shutdown().await;
}

#[tokio::test]
async fn disabled_mouth_driver_reports_failure_instead_of_acknowledging_setup() {
    let driver = AvatarDriver::start(Default::default()).unwrap();
    let selected = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        driver.mouth_parameter().select("OtherMouth".into()),
    )
    .await;
    assert!(selected.unwrap().is_err());
    driver.shutdown().await;
}
