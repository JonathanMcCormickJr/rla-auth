use crate::{
    app_state::{AppState, BannedTokenStoreType, UserStoreType}, domain::{
        AuthAPIError, data_stores::{LoginAttemptId, TwoFACode, UserStoreError}, email::Email, password::Password
    }, services::TwoFACodeStore, utils::auth::generate_auth_cookie
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::CookieJar;
use serde::{ Deserialize, Serialize };

pub async fn login(
    State(state): State<AppState<UserStoreType, BannedTokenStoreType>>,
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

    let user = match user_store.get_user(&email) {
        Ok(user) => user,
        Err(error) => {
            let auth_error = match error {
                UserStoreError::UserNotFound => AuthAPIError::InvalidCredentials,
                UserStoreError::MalformedCredentials => AuthAPIError::MalformedCredentials,
                UserStoreError::InvalidCredentials => AuthAPIError::AuthenticationFailed,
                UserStoreError::UnexpectedError => AuthAPIError::UnexpectedError,
                UserStoreError::UserAlreadyExists => AuthAPIError::UnexpectedError,
            };

            return (jar, Err(auth_error));
        }
    };

    if user.password != password {
        return (jar, Err(AuthAPIError::AuthenticationFailed));
    }

    match user.requires_2fa {
        true => handle_2fa(&user.email, &state, jar).await,
        false => handle_no_2fa(&user.email, jar).await,
    }
}

async fn handle_2fa(
    email: &Email,
    state: &AppState<UserStoreType, BannedTokenStoreType>,
    jar: CookieJar,
) -> (
    CookieJar,
    Result<(StatusCode, Json<LoginResponse>), AuthAPIError>,
) {
    // First, we must generate a new random login attempt ID and 2FA code
    let login_attempt_id = LoginAttemptId::default();
    let two_fa_code = TwoFACode::default();
    let mut two_fa_code_store = state.two_fa_code_store.write().await;

    if let Err(_) = two_fa_code_store
        .add_code(email.clone(), login_attempt_id.clone(), two_fa_code.clone())
        .await
    {
        return (jar, Err(AuthAPIError::UnexpectedError));
    }

    if let Err(_) = state.email_client.send_email(email, "Your 2FA Code", &two_fa_code.as_ref()).await {
        return (jar, Err(AuthAPIError::UnexpectedError));
    }

    let response = LoginResponse::TwoFactorAuth(TwoFactorAuthResponse {
        message: "2FA required".to_string(),
        login_attempt_id: login_attempt_id.as_ref().to_owned(),
    });

    (jar, Ok((StatusCode::PARTIAL_CONTENT, Json(response))))
}

async fn handle_no_2fa(
    email: &Email,
    jar: CookieJar,
) -> (
    CookieJar,
    Result<(StatusCode, Json<LoginResponse>), AuthAPIError>,
) {
    let auth_cookie = match generate_auth_cookie(email) {
        Ok(cookie) => cookie,
        Err(_) => return (jar, Err(AuthAPIError::UnexpectedError)),
    };

    let updated_jar = jar.add(auth_cookie);
    (updated_jar, Ok((StatusCode::OK, Json(LoginResponse::RegularAuth))))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum LoginResponse {
    RegularAuth,
    TwoFactorAuth(TwoFactorAuthResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwoFactorAuthResponse {
    pub message: String,
    #[serde(rename = "loginAttemptId")]
    pub login_attempt_id: String,
}
