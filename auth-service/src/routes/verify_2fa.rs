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
use serde::Deserialize;

pub async fn verify_2fa(
    State(state): State<AppState<UserStoreType, BannedTokenStoreType>>,
    jar: CookieJar,
    Json(request): Json<Verify2FARequest>,
) -> (CookieJar, Result<impl IntoResponse, AuthAPIError>) {
    let email = match Email::parse(&request.email) {
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
                    Err(UserStoreError::UnexpectedError)
                    | Err(UserStoreError::UserAlreadyExists) => {
                        (jar, Err(AuthAPIError::UnexpectedError))
                    }
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

        if two_fa_code_store.remove_code(&email).await.is_err() {
            return (jar, Err(AuthAPIError::UnexpectedError));
        }
    }

    let auth_cookie = match generate_auth_cookie(&email) {
        Ok(cookie) => cookie,
        Err(_) => return (jar, Err(AuthAPIError::UnexpectedError)),
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
        get_postgres_pool,
        services::data_stores::{
            hashmap_2fa_code_store::HashmapTwoFACodeStore,
            hashset_banned_token_store::HashsetBannedTokenStore,
            mock_email_client::MockEmailClient,
            postgres_user_store::PostgresUserStore,
        },
        utils::constants::{DATABASE_URL, JWT_COOKIE_NAME},
    };
    use axum::Json;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    async fn test_state() -> AppState<UserStoreType, BannedTokenStoreType> {
        let pg_pool = get_postgres_pool(&DATABASE_URL)
            .await
            .expect("Failed to create Postgres connection pool for tests");
        let user_store = PostgresUserStore::new(pg_pool);
        AppState::new(
            Arc::new(RwLock::new(user_store)) as UserStoreType,
            HashsetBannedTokenStore::default(),
            HashmapTwoFACodeStore::default(),
            Arc::new(MockEmailClient) as EmailClientType,
        )
    }

    #[tokio::test]
    async fn should_verify_stored_2fa_code_and_issue_auth_cookie() {
        let state = test_state().await;
        let jar = CookieJar::new();
        let email = Email::parse("test@example.com").expect("valid email");
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
            email: email.as_ref().to_owned(),
            login_attempt_id: login_attempt_id.as_ref().to_owned(),
            code: two_fa_code.as_ref().to_owned(),
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
        let email = Email::parse("test@example.com").expect("valid email");
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
            email: email.as_ref().to_owned(),
            login_attempt_id: login_attempt_id.as_ref().to_owned(),
            code: invalid_two_fa_code.as_ref().to_owned(),
        };

        let (_, response) = verify_2fa(State(state), jar, Json(request)).await;
        let response = match response {
            Ok(response) => response.into_response(),
            Err(_) => panic!("mismatched code should return an unauthorized response"),
        };

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
