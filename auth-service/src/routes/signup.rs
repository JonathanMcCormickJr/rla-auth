use crate::{
    app_state::{AppState, BannedTokenStoreType, UserStoreType},
    domain::{data_stores::UserStore, email::Email, password::HashedPassword, AuthAPIError, User},
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};


#[tracing::instrument(name = "Signup", skip_all)]
pub async fn signup(
    State(state): State<AppState<UserStoreType, BannedTokenStoreType>>,
    Json(request): Json<SignupRequest>,
) -> Result<impl IntoResponse, AuthAPIError> {
    let user = User {
        uuid: uuid::Uuid::new_v4(),
        email: Email::parse(request.email).map_err(|_| AuthAPIError::InvalidCredentials)?,
        password: HashedPassword::parse(request.password)
            .await
            .map_err(|_| AuthAPIError::InvalidCredentials)?,
        requires_2fa: request.requires_2fa,
    };

    let mut user_store = state.user_store.write().await;

    if UserStore::get_user(&*user_store, &user.email).await.is_ok() {
        return Err(AuthAPIError::UserAlreadyExists);
    }

    if let Err(e) = UserStore::add_user(&mut *user_store, user).await {
        return Err(AuthAPIError::UnexpectedError(e.into()));
    }

    let response = Json(SignupResponse {
        message: "User created successfully!".to_string(),
    });

    Ok((StatusCode::CREATED, response))
}

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: SecretString,
    pub password: SecretString,
    #[serde(rename = "requires2FA")]
    pub requires_2fa: bool,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SignupResponse {
    pub message: String,
}
