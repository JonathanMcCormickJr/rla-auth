use crate::domain::{ data_stores::UserStore, email::Email, password::Password};
use axum::{Json, response::IntoResponse};
use reqwest::StatusCode;

pub async fn login(user_store: impl UserStore, Json(body): Json<serde_json::Value>) -> impl IntoResponse {
    // JSON object must have username and password fields, otherwise return 422 Unprocessable Entity. Use the `parse` functions in email.rs and password.rs to validate the fields.
    let email = body.get("email").and_then(|v| v.as_str());
            let password = body.get("password").and_then(|v| v.as_str());
    if email.is_none() || password.is_none() {
        return (StatusCode::UNPROCESSABLE_ENTITY, "Missing email or password").into_response();
    }
    let email = email.unwrap();
    let password = password.unwrap();
    if Email::parse(email).is_err() {
        return (StatusCode::UNPROCESSABLE_ENTITY, "Invalid email").into_response();
    }
    if Password::parse(password).is_err() {
        return (StatusCode::UNPROCESSABLE_ENTITY, "Invalid password").into_response();
    }

    // Check if the email and password match a user in the database. If not, return 400 Bad Request with an appropriate error message.
    if user_store.get_user(&Email::parse(email).unwrap()).await.is_err() {
        return (StatusCode::BAD_REQUEST, "Invalid email or password").into_response();
    }

    (StatusCode::OK, "Login successful").into_response()
}
