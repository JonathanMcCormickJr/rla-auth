use crate::{
    app_state::{AppState, UserStoreType},
    domain::{email::Email, password::Password, AuthAPIError},
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

pub async fn login(
    State(state): State<AppState<UserStoreType>>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AuthAPIError> {
    let email = body
        .get("email")
        .and_then(|value| value.as_str())
        .ok_or(AuthAPIError::MalformedCredentials)?;
    let password = body
        .get("password")
        .and_then(|value| value.as_str())
        .ok_or(AuthAPIError::MalformedCredentials)?;

    let email = Email::parse(email).map_err(|_| AuthAPIError::MalformedCredentials)?;

    // Login accepts any provided password string; authentication outcome is handled below.
    let password = Password::parse(password).map_err(|_| AuthAPIError::AuthenticationFailed)?;

    let user_store = state.user_store.read().await;

    user_store
        .validate_user(&email, &password)
        .map_err(|error| match error {
            crate::domain::data_stores::UserStoreError::UserNotFound => {
                AuthAPIError::InvalidCredentials
            }
            crate::domain::data_stores::UserStoreError::InvalidCredentials => {
                AuthAPIError::AuthenticationFailed
            }
            crate::domain::data_stores::UserStoreError::MalformedCredentials => {
                AuthAPIError::MalformedCredentials
            }
            _ => AuthAPIError::UnexpectedError,
        })?;

    Ok((StatusCode::OK, "Login successful"))
}
