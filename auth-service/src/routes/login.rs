use crate::{
    app_state::{AppState, UserStoreType},
    domain::{email::Email, password::Password, AuthAPIError},
    utils::auth::generate_auth_cookie,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::CookieJar;
use serde::Deserialize;

pub async fn login(
    State(state): State<AppState<UserStoreType>>,
    jar: CookieJar,
    Json(request): Json<LoginRequest>,
) -> (CookieJar, Result<impl IntoResponse, AuthAPIError>) {
    let email = match Email::parse(&request.email) {
        Ok(email) => email,
        Err(_) => return (jar, Err(AuthAPIError::MalformedCredentials)),
    };

    // Map password parse failures to authentication errors to avoid leaking details.
    let password = match Password::parse(&request.password) {
        Ok(password) => password,
        Err(_) => return (jar, Err(AuthAPIError::AuthenticationFailed)),
    };

    let user_store = state.user_store.read().await;

    if let Err(error) = user_store.validate_user(&email, &password) {
        let auth_error = match error {
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
        };

        return (jar, Err(auth_error));
    }

    let auth_cookie = match generate_auth_cookie(&email) {
        Ok(cookie) => cookie,
        Err(_) => return (jar, Err(AuthAPIError::UnexpectedError)),
    };

    let updated_jar = jar.add(auth_cookie);

    (updated_jar, Ok(StatusCode::OK.into_response()))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}
