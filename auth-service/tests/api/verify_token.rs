use crate::helpers::TestApp;
use auth_service::{
    domain::{data_stores::TwoFACodeStore, Email},
    routes::TwoFactorAuthResponse,
};
use auth_service::utils::constants::JWT_COOKIE_NAME;
use serde_json::json;

#[tokio::test]
async fn should_return_422_if_malformed_input() {
    let mut app = TestApp::new().await;

    let body = "not a valid JSON body";

    let response = app.post_verify_token(&body).await;

    assert_eq!(response.status().as_u16(), 422);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_400_if_token_is_missing() {
    let mut app = TestApp::new().await;

    let body = json!({
        "token": "   "
    });

    let response = app.post_verify_token(&body).await;

    assert_eq!(response.status().as_u16(), 400);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_401_if_token_is_invalid() {
    let mut app = TestApp::new().await;

    let body = json!({
        "token": "not.a.valid.jwt"
    });

    let response = app.post_verify_token(&body).await;

    assert_eq!(response.status().as_u16(), 401);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_200_if_token_is_valid() {
    let mut app = TestApp::new().await;

    let email = crate::helpers::get_random_email();
    let password = "Password123!";

    let signup_body = json!({
        "email": email,
        "password": password,
        "requires2FA": false,
    });

    let signup_response = app.post_signup_with_body(&signup_body).await;
    assert_eq!(signup_response.status().as_u16(), 201);

    let login_body = json!({
        "email": signup_body["email"],
        "password": password,
    });

    let login_response = app.post_login(&login_body).await;
    assert_eq!(login_response.status().as_u16(), 200);

    let token = login_response
        .cookies()
        .find(|cookie| cookie.name() == JWT_COOKIE_NAME)
        .expect("No auth cookie found")
        .value()
        .to_owned();

    let verify_body = json!({ "token": token });
    let verify_response = app.post_verify_token(&verify_body).await;

    assert_eq!(verify_response.status().as_u16(), 200);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_401_if_banned_token() {
    let mut app = TestApp::new().await;

    let email = crate::helpers::get_random_email();
    let password = "Password123!";

    let signup_body = json!({
        "email": email,
        "password": password,
        "requires2FA": false,
    });

    let signup_response = app.post_signup_with_body(&signup_body).await;
    assert_eq!(signup_response.status().as_u16(), 201);

    let login_body = json!({
        "email": signup_body["email"],
        "password": password,
    });

    let login_response = app.post_login(&login_body).await;
    assert_eq!(login_response.status().as_u16(), 200);

    let token = login_response
        .cookies()
        .find(|cookie| cookie.name() == JWT_COOKIE_NAME)
        .expect("No auth cookie found")
        .value()
        .to_owned();

    // Ban the token by logging out, which writes it to the banned token store.
    let ban_response = app.post_logout().await;
    assert_eq!(ban_response.status().as_u16(), 200);

    // Now the token should be banned, so verification should fail with a 401
    let verify_body = json!({ "token": token });
    let verify_response = app.post_verify_token(&verify_body).await;

    assert_eq!(verify_response.status().as_u16(), 401);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_200_if_correct_code() {
    // Make sure to assert the auth cookie gets set
    let mut app = TestApp::new().await;

    let email = crate::helpers::get_random_email();
    let password = "Password123!";

    let signup_body = json!({
        "email": email,
        "password": password,
        "requires2FA": true,
    });

    let signup_response = app.post_signup_with_body(&signup_body).await;
    assert_eq!(signup_response.status().as_u16(), 201);

    let login_body = json!({
        "email": signup_body["email"],
        "password": password,
    });

    let login_response = app.post_login(&login_body).await;
    assert_eq!(login_response.status().as_u16(), 206);

    let challenge: TwoFactorAuthResponse = login_response
        .json()
        .await
        .expect("2FA challenge body should be returned");

    let parsed_email = Email::parse(&email).expect("signup email should be valid");
    let (_, code) = app
        .app_state
        .two_fa_code_store
        .read()
        .await
        .get_code(&parsed_email)
        .await
        .expect("2FA code should exist after login challenge");

    let verify_body = json!({
        "email": parsed_email.as_ref(),
        "login_attempt_id": challenge.login_attempt_id,
        "code": code.as_ref(),
    });

    let verify_response = app.post_verify_2fa(&verify_body).await;

    assert_eq!(verify_response.status().as_u16(), 200);

    let auth_cookie = verify_response
        .cookies()
        .find(|cookie| cookie.name() == JWT_COOKIE_NAME)
        .expect("No auth cookie found after successful 2FA verification");

    assert!(!auth_cookie.value().is_empty());

    app.clean_up().await;
}