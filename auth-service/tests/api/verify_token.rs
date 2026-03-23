use crate::helpers::TestApp;
use auth_service::utils::constants::JWT_COOKIE_NAME;
use serde_json::json;

#[tokio::test]
async fn should_return_422_if_malformed_input() {
    let app = TestApp::new().await;

    let body = "not a valid JSON body";

    let response = app.post_verify_token(&body).await;

    assert_eq!(response.status().as_u16(), 422);
}

#[tokio::test]
async fn should_return_400_if_token_is_missing() {
    let app = TestApp::new().await;

    let body = json!({
        "token": "   "
    });

    let response = app.post_verify_token(&body).await;

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn should_return_401_if_token_is_invalid() {
    let app = TestApp::new().await;

    let body = json!({
        "token": "not.a.valid.jwt"
    });

    let response = app.post_verify_token(&body).await;

    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn should_return_200_if_token_is_valid() {
    let app = TestApp::new().await;

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
}

#[tokio::test]
async fn should_return_401_if_banned_token() {
    let app = TestApp::new().await;

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
}