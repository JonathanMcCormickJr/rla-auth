use crate::{
    app_state::{AppState, BannedTokenStoreType, UserStoreType},
    domain::{
        data_stores::LoginAttemptId, data_stores::TwoFACode, data_stores::UserStore,
        data_stores::UserStoreError, email::Email, AuthAPIError,
    },
    services::TwoFACodeStore,
    utils::auth::generate_auth_cookie,
    ErrorResponse,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::CookieJar;
use color_eyre::eyre::eyre;
#[cfg_attr(not(test), allow(unused_imports))]
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[tracing::instrument(name = "Verify 2FA", skip_all)]
pub async fn verify_2fa(
    State(state): State<AppState<UserStoreType, BannedTokenStoreType>>,
    jar: CookieJar,
    Json(request): Json<Verify2FARequest>,
) -> (CookieJar, Result<impl IntoResponse, AuthAPIError>) {
    let email = match Email::parse(SecretString::new(request.email.into_boxed_str())) {
        Ok(email) => email,
        Err(_) => return (jar, Err(AuthAPIError::MalformedCredentials)),
    };
    let login_attempt_id = match LoginAttemptId::parse(request.login_attempt_id) {
        Ok(login_attempt_id) => login_attempt_id,
        Err(_) => return (jar, Err(AuthAPIError::MalformedCredentials)),
    };
    let two_fa_code = match TwoFACode::parse(request.code) {
        Ok(two_fa_code) => two_fa_code,
        Err(_) => return (jar, Err(AuthAPIError::MalformedCredentials)),
    };

    let (stored_login_attempt_id, stored_two_fa_code) = {
        let two_fa_code_store = state.two_fa_code_store.read().await;

        match two_fa_code_store.get_code(&email).await {
            Ok(stored_code) => stored_code,
            Err(_) => {
                // If the user exists but no active challenge is found, treat it as replay/expired credentials.
                let user_store = state.user_store.read().await;
                return match UserStore::get_user(&*user_store, &email).await {
                    Ok(_) => {
                        let response = (
                            StatusCode::UNAUTHORIZED,
                            Json(ErrorResponse {
                                error: "Invalid credentials".to_string(),
                            }),
                        )
                            .into_response();

                        (jar, Ok(response))
                    }
                    Err(UserStoreError::UserNotFound)
                    | Err(UserStoreError::MalformedCredentials)
                    | Err(UserStoreError::InvalidCredentials) => {
                        (jar, Err(AuthAPIError::InvalidCredentials))
                    }
                    Err(UserStoreError::UnexpectedError(report)) => {
                        (jar, Err(AuthAPIError::UnexpectedError(report)))
                    }
                    Err(UserStoreError::UserAlreadyExists) => (
                        jar,
                        Err(AuthAPIError::UnexpectedError(eyre!(
                            "Unexpected UserAlreadyExists from get_user during verify_2fa"
                        ))),
                    ),
                };
            }
        }
    };

    if stored_login_attempt_id != login_attempt_id || stored_two_fa_code != two_fa_code {
        let response = (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid credentials".to_string(),
            }),
        )
            .into_response();

        return (jar, Ok(response));
    }

    {
        let mut two_fa_code_store = state.two_fa_code_store.write().await;

        if let Err(e) = two_fa_code_store.remove_code(&email).await {
            return (jar, Err(AuthAPIError::UnexpectedError(e.into())));
        }
    }

    let auth_cookie = match generate_auth_cookie(&email) {
        Ok(cookie) => cookie,
        Err(e) => return (jar, Err(AuthAPIError::UnexpectedError(e))),
    };

    (jar.add(auth_cookie), Ok(StatusCode::OK.into_response()))
}

#[derive(Deserialize)]
pub struct Verify2FARequest {
    pub email: String,
    #[serde(alias = "loginAttemptId")]
    pub login_attempt_id: String,
    #[serde(alias = "2FACode")]
    pub code: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app_state::{EmailClientType, UserStoreType},
        domain::data_stores::TwoFACodeStoreError,
        get_postgres_pool, get_redis_client,
        services::data_stores::{
            mock_email_client::MockEmailClient,
            postgres_user_store::PostgresUserStore,
            redis_banned_token_store::RedisBannedTokenStore,
            redis_two_fa_code_store::RedisTwoFACodeStore,
        },
        utils::constants::{DATABASE_URL, JWT_COOKIE_NAME, REDIS_HOST_NAME},
    };
    use axum::Json;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn redis_connection() -> redis::Connection {
        get_redis_client(REDIS_HOST_NAME.to_owned())
            .expect("Failed to open Redis client for tests")
            .get_connection()
            .expect("Failed to get Redis connection for tests")
    }

    async fn test_state() -> AppState<UserStoreType, BannedTokenStoreType> {
        let pg_pool = get_postgres_pool(&DATABASE_URL)
            .await
            .expect("Failed to create Postgres connection pool for tests");
        let user_store = PostgresUserStore::new(pg_pool);
        let banned_token_store =
            RedisBannedTokenStore::new(Arc::new(RwLock::new(redis_connection())));
        let two_fa_code_store =
            RedisTwoFACodeStore::new(Arc::new(RwLock::new(redis_connection())));
        AppState::new(
            Arc::new(RwLock::new(user_store)) as UserStoreType,
            banned_token_store,
            two_fa_code_store,
            Arc::new(MockEmailClient) as EmailClientType,
        )
    }

    #[tokio::test]
    async fn should_verify_stored_2fa_code_and_issue_auth_cookie() {
        let state = test_state().await;
        let jar = CookieJar::new();
        let email = Email::parse(SecretString::new(
            "test@example.com".to_owned().into_boxed_str(),
        ))
        .expect("valid email");
        let login_attempt_id = LoginAttemptId::default();
        let two_fa_code = TwoFACode::default();

        state
            .two_fa_code_store
            .write()
            .await
            .add_code(email.clone(), login_attempt_id.clone(), two_fa_code.clone())
            .await
            .expect("2FA code should be stored");

        let request = Verify2FARequest {
            email: email.as_ref().expose_secret().to_owned(),
            login_attempt_id: login_attempt_id.as_ref().expose_secret().to_owned(),
            code: two_fa_code.as_ref().expose_secret().to_owned(),
        };

        let (updated_jar, response) = verify_2fa(State(state.clone()), jar, Json(request)).await;
        let response = match response {
            Ok(response) => response.into_response(),
            Err(_) => panic!("verification should succeed"),
        };

        assert_eq!(response.status(), StatusCode::OK);

        let auth_cookie = updated_jar
            .get(JWT_COOKIE_NAME)
            .expect("auth cookie should be set after successful 2FA");

        assert!(!auth_cookie.value().is_empty());

        let store_result = state.two_fa_code_store.read().await.get_code(&email).await;

        assert!(matches!(
            store_result,
            Err(TwoFACodeStoreError::LoginAttemptIdNotFound)
        ));
    }

    #[tokio::test]
    async fn should_return_401_when_stored_2fa_data_does_not_match() {
        let state = test_state().await;
        let jar = CookieJar::new();
        let email = Email::parse(SecretString::new(
            "test@example.com".to_owned().into_boxed_str(),
        ))
        .expect("valid email");
        let login_attempt_id = LoginAttemptId::default();
        let stored_two_fa_code = TwoFACode::parse("123456".to_string()).expect("valid code");
        let invalid_two_fa_code = TwoFACode::parse("654321".to_string()).expect("valid code");

        state
            .two_fa_code_store
            .write()
            .await
            .add_code(email.clone(), login_attempt_id.clone(), stored_two_fa_code)
            .await
            .expect("2FA code should be stored");

        let request = Verify2FARequest {
            email: email.as_ref().expose_secret().to_owned(),
            login_attempt_id: login_attempt_id.as_ref().expose_secret().to_owned(),
            code: invalid_two_fa_code.as_ref().expose_secret().to_owned(),
        };

        let (_, response) = verify_2fa(State(state), jar, Json(request)).await;
        let response = match response {
            Ok(response) => response.into_response(),
            Err(_) => panic!("mismatched code should return an unauthorized response"),
        };

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
