use auth_service::domain::data_stores::BannedTokenStore;
use auth_service::utils::constants::JWT_COOKIE_NAME;
use reqwest::cookie::CookieStore;
use reqwest::Url;
use serde_json::json;

use crate::helpers::{get_random_email, TestApp};

#[tokio::test]
async fn should_return_400_if_jwt_cookie_missing() {
    let mut app = TestApp::new().await;

    let response = app.post_logout().await;

    assert_eq!(response.status().as_u16(), 400);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_401_if_invalid_token() {
    let mut app = TestApp::new().await;

    // add invalid cookie
    app.cookie_jar.add_cookie_str(
        &format!(
            "{}=invalid; HttpOnly; SameSite=Lax; Path=/",
            JWT_COOKIE_NAME
        ),
        &Url::parse(&app.address).expect("Failed to parse URL"),
    );

    let response = app.post_logout().await;

    assert_eq!(response.status().as_u16(), 401);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_200_if_valid_jwt_cookie() {
    let mut app = TestApp::new().await;

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

    // Now we can call the log-in route with the correct credentials and assert that a 200 status code is returned.
    let login_body = json!({
        "email": email,
        "password": password,
    });
    let login_response = app.post_login(&login_body).await;
    assert_eq!(login_response.status().as_u16(), 200);

    let app_url = Url::parse(&app.address).expect("Failed to parse app URL");
    let cookie_header = app
        .cookie_jar
        .cookies(&app_url)
        .expect("Expected auth cookie to be present after login")
        .to_str()
        .expect("Cookie header should be valid UTF-8")
        .to_string();

    let jwt_token = cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{}=", JWT_COOKIE_NAME)))
        .expect("Expected JWT cookie in jar")
        .to_string();

    // Finally, we can call the log-out route and assert that a 200 status code is returned.
    let logout_response = app.post_logout().await;
    assert_eq!(logout_response.status().as_u16(), 200);

    let banned_tokens = app.app_state.banned_token_store.read().await;
    assert!(banned_tokens
        .is_token_banned(&jwt_token)
        .await
        .expect("Failed to inspect banned token store"));
    drop(banned_tokens);

    app.clean_up().await;
}

#[tokio::test]
async fn should_return_400_if_logout_called_twice_in_a_row() {
    let mut app = TestApp::new().await;

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

    // Now we can call the log-in route with the correct credentials and assert that a 200 status code is returned.
    let login_body = json!({
        "email": email,
        "password": password,
    });
    let login_response = app.post_login(&login_body).await;
    assert_eq!(login_response.status().as_u16(), 200);

    // Call the log-out route twice in a row and assert that the second call returns a 400 status code.
    let first_logout_response = app.post_logout().await;
    assert_eq!(first_logout_response.status().as_u16(), 200);

    let second_logout_response = app.post_logout().await;
    assert_eq!(second_logout_response.status().as_u16(), 400);

    app.clean_up().await;
}
