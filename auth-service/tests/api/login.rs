use crate::helpers::{get_random_email, TestApp};
use auth_service::{routes::TwoFactorAuthResponse, utils::constants::JWT_COOKIE_NAME};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn should_return_422_if_malformed_credentials() {
    let app = TestApp::new().await;

    let body = json!({
        "email": "not-an-email",
        "password": "short",
    });

    let response = app.post_login(&body).await;

    assert_eq!(response.status().as_u16(), 422);
}

#[tokio::test]
async fn should_return_400_if_invalid_input() {
    // Call the log-in route with invalid credentials and assert that a
    // 400 HTTP status code is returned along with the appropriate error message. 
    let app = TestApp::new().await;
    let body = json!({
        "email": get_random_email(),
        "password": "shor20489J4#t",
    });
    let response = app.post_login(&body).await;
    assert_eq!(response.status().as_u16(), 400);
}


#[tokio::test]
async fn should_return_401_if_incorrect_credentials() {
    // Call the log-in route with incorrect credentials and assert
    // that a 401 HTTP status code is returned along with the appropriate error message.     
    let app = TestApp::new().await;

    // First, we need to create a user with known credentials. We can do this by calling the sign-up route.
    let email = get_random_email();
    let password = "shScSor20489J4#t";
    let signup_body = json!({
        "email": email,
        "password": password,
        "requires2FA": false,
    });
    let real_response = app.post_signup_with_body(&signup_body).await;
    assert_eq!(real_response.status().as_u16(), 201);

    // Now we can call the log-in route with the correct email but incorrect password and assert that a 401 status code is returned.
    let body = json!({
        "email": email,
        "password": "incorrect-password",
    });
    let response = app.post_login(&body).await;
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn should_return_200_if_valid_credentials_and_2fa_disabled() {
    let app = TestApp::new().await;

    let random_email = get_random_email();

    let signup_body = serde_json::json!({
        "email": random_email,
        "password": "Password123!",
        "requires2FA": false
    });

    let response = app.post_signup_with_body(&signup_body).await;

    assert_eq!(response.status().as_u16(), 201);

    let login_body = serde_json::json!({
        "email": random_email,
        "password": "Password123!",
    });

    let response = app.post_login(&login_body).await;

    assert_eq!(response.status().as_u16(), 200);

    let auth_cookie = response
        .cookies()
        .find(|cookie| cookie.name() == JWT_COOKIE_NAME)
        .expect("No auth cookie found");

    assert!(!auth_cookie.value().is_empty());
}

#[tokio::test]
async fn should_return_206_if_valid_credentials_and_2fa_enabled() {
    let app = TestApp::new().await;

    let random_email = get_random_email();

    let signup_body = serde_json::json!({
        "email": random_email,
        "password": "Password123!",
        "requires2FA": true
    });

    let signup_response = app.post_signup_with_body(&signup_body).await;

    assert_eq!(signup_response.status().as_u16(), 201);

    let login_body = serde_json::json!({
        "email": random_email,
        "password": "Password123!",
    });

    let response = app.post_login(&login_body).await;

    assert_eq!(response.status().as_u16(), 206);

    let auth_cookie = response
        .cookies()
        .find(|cookie| cookie.name() == JWT_COOKIE_NAME);

    assert!(auth_cookie.is_none());

    let body: TwoFactorAuthResponse = response.json().await.expect("login response body");

    assert_eq!(body.message, "2FA required");

    let first_attempt = Uuid::parse_str(body.login_attempt_id.as_str())
        .expect("loginAttemptId should be a valid UUID");

    assert_eq!(first_attempt.get_version_num(), 4);

    let second_response = app.post_login(&login_body).await;

    assert_eq!(second_response.status().as_u16(), 206);

    let second_auth_cookie = second_response
        .cookies()
        .find(|cookie| cookie.name() == JWT_COOKIE_NAME);

    assert!(second_auth_cookie.is_none());

    let second_body: serde_json::Value = second_response
        .json()
        .await
        .expect("second login response body");

    assert_eq!(second_body["message"], json!("2FA required"));

    let second_attempt = second_body["loginAttemptId"]
        .as_str()
        .expect("loginAttemptId should be a string")
        .to_owned();

    let second_attempt = Uuid::parse_str(&second_attempt)
        .expect("second loginAttemptId should be a valid UUID");

    assert_eq!(second_attempt.get_version_num(), 4);

    assert!(
        second_attempt != first_attempt,
        "expected loginAttemptId to be unique across attempts: first={first_attempt}, second={second_attempt}"
    );

    assert_eq!(
        body.message,
        "2FA required".to_string()
    );

    let json_body = body;

    assert_eq!(json_body.message, "2FA required".to_owned());

    json_body.login_attempt_id.parse::<Uuid>().expect("loginAttemptId should be a valid UUID");

}
