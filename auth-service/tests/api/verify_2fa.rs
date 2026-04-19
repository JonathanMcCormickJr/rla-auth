use crate::helpers::TestApp;
use auth_service::{
    domain::{data_stores::TwoFACodeStore, Email},
    routes::TwoFactorAuthResponse,
    ErrorResponse,
};
use serde_json::json;

#[tokio::test]
async fn should_return_422_if_malformed_input() {
    let mut app = TestApp::new().await;

    let body = serde_json::json!({
        "email": "not-an-email",
        "login_attempt_id": "not-a-uuid",
        "code": "123456"
    });

    let response = app.post_verify_2fa(&body).await;

    assert_eq!(response.status().as_u16(), 422);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_400_if_invalid_input() {
    let mut app = TestApp::new().await;

    let body = serde_json::json!({
        "email": "invalid-email@example.com",
        "login_attempt_id": "00000000-0000-0000-0000-000000000000",
        "code": "123456"
    });

    let response = app.post_verify_2fa(&body).await;

    assert_eq!(response.status().as_u16(), 400);

    app.clean_up().await;
}

#[tokio::test]
async fn should_accept_frontend_field_names() {
    let mut app = TestApp::new().await;

    let body = serde_json::json!({
        "email": "invalid-email@example.com",
        "loginAttemptId": "00000000-0000-0000-0000-000000000000",
        "2FACode": "123456"
    });

    let response = app.post_verify_2fa(&body).await;

    assert_eq!(response.status().as_u16(), 400);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_401_if_incorrect_credentials() {
    let mut app = TestApp::new().await;

    let email = crate::helpers::get_random_email();
    let password = "Password123!";

    let signup_body = json!({
        "email": email,
        "password": password,
        "requires2FA": true
    });

    let signup_response = app.post_signup_with_body(&signup_body).await;
    assert_eq!(signup_response.status().as_u16(), 201);

    let login_body = json!({
        "email": email,
        "password": password,
    });

    let login_response = app.post_login(&login_body).await;
    assert_eq!(login_response.status().as_u16(), 206);

    let login_response_body: TwoFactorAuthResponse = login_response
        .json()
        .await
        .expect("login should return a 2FA challenge body");

    let email = Email::parse(&email).expect("signup email should be valid");
    let (_, stored_code) = app
        .app_state
        .two_fa_code_store
        .read()
        .await
        .get_code(&email)
        .await
        .expect("2FA code should exist after login challenge");

    let invalid_code = if stored_code.as_ref() == "000000" {
        "999999"
    } else {
        "000000"
    };

    let verify_body = json!({
        "email": email.as_ref(),
        "login_attempt_id": login_response_body.login_attempt_id,
        "code": invalid_code
    });

    let verify_response = app.post_verify_2fa(&verify_body).await;

    assert_eq!(verify_response.status().as_u16(), 401);

    let error: ErrorResponse = verify_response
        .json()
        .await
        .expect("error body should be returned");

    assert_eq!(error.error, "Invalid credentials");

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_401_if_old_code() {
    // Call login twice. Then use the first challenge's code and attempt id, which should be invalid.
    let mut app = TestApp::new().await;

    let email = crate::helpers::get_random_email();
    let password = "Password123!";

    let signup_body = json!({
        "email": email,
        "password": password,
        "requires2FA": true
    });

    let signup_response = app.post_signup_with_body(&signup_body).await;
    assert_eq!(signup_response.status().as_u16(), 201);

    let login_body = json!({
        "email": email,
        "password": password,
    });

    let first_login_response = app.post_login(&login_body).await;
    assert_eq!(first_login_response.status().as_u16(), 206);
    let first_challenge: TwoFactorAuthResponse = first_login_response
        .json()
        .await
        .expect("first login should return a 2FA challenge body");

    let email = Email::parse(&email).expect("signup email should be valid");
    let (_, first_code) = app
        .app_state
        .two_fa_code_store
        .read()
        .await
        .get_code(&email)
        .await
        .expect("first login should create a code");

    let second_login_response = app.post_login(&login_body).await;
    assert_eq!(second_login_response.status().as_u16(), 206);
    let second_challenge: TwoFactorAuthResponse = second_login_response
        .json()
        .await
        .expect("second login should return a 2FA challenge body");

    assert_ne!(
        first_challenge.login_attempt_id, second_challenge.login_attempt_id,
        "a second login should replace the previous challenge"
    );

    let old_verify_body = json!({
        "email": email.as_ref(),
        "login_attempt_id": first_challenge.login_attempt_id,
        "code": first_code.as_ref()
    });

    let old_verify_response = app.post_verify_2fa(&old_verify_body).await;
    assert_eq!(old_verify_response.status().as_u16(), 401);

    let error: ErrorResponse = old_verify_response
        .json()
        .await
        .expect("error body should be returned");
    assert_eq!(error.error, "Invalid credentials");

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_401_if_same_code_twice() {
    let mut app = TestApp::new().await;

    let email = crate::helpers::get_random_email();
    let password = "Password123!";

    let signup_body = json!({
        "email": email,
        "password": password,
        "requires2FA": true
    });

    let signup_response = app.post_signup_with_body(&signup_body).await;
    assert_eq!(signup_response.status().as_u16(), 201);

    let login_body = json!({
        "email": email,
        "password": password,
    });

    let login_response = app.post_login(&login_body).await;
    assert_eq!(login_response.status().as_u16(), 206);

    let challenge: TwoFactorAuthResponse = login_response
        .json()
        .await
        .expect("login should return a 2FA challenge body");

    let email = Email::parse(&email).expect("signup email should be valid");
    let (_, code) = app
        .app_state
        .two_fa_code_store
        .read()
        .await
        .get_code(&email)
        .await
        .expect("2FA code should exist after login challenge");

    let verify_body = json!({
        "email": email.as_ref(),
        "login_attempt_id": challenge.login_attempt_id,
        "code": code.as_ref(),
    });

    let first_verify_response = app.post_verify_2fa(&verify_body).await;
    assert_eq!(first_verify_response.status().as_u16(), 200);

    // Replay the exact same combination; this must be rejected as one-time credentials.
    let second_verify_response = app.post_verify_2fa(&verify_body).await;
    assert_eq!(second_verify_response.status().as_u16(), 401);

    let error: ErrorResponse = second_verify_response
        .json()
        .await
        .expect("error body should be returned");
    assert_eq!(error.error, "Invalid credentials");

    app.clean_up().await;
}