use crate::{
    app_state::{AppState, BannedTokenStoreType, UserStoreType},
    domain::{data_stores::UserStore, email::Email, password::HashedPassword, AuthAPIError, User},
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

pub async fn signup(
    State(state): State<AppState<UserStoreType, BannedTokenStoreType>>,
    Json(request): Json<SignupRequest>,
) -> Result<impl IntoResponse, AuthAPIError> {
    // Create a new `User` instance using data in the `request`
    let user = User {
        uuid: uuid::Uuid::new_v4(),
        email: Email::parse(&request.email).map_err(|_| AuthAPIError::InvalidCredentials)?,
        password: HashedPassword::parse(request.password.clone())
            .await
            .map_err(|_| AuthAPIError::InvalidCredentials)?,
        requires_2fa: request.requires_2fa,
    };

    let mut user_store = state.user_store.write().await;

    if UserStore::get_user(&*user_store, &user.email).await.is_ok() {
        return Err(AuthAPIError::UserAlreadyExists);
    }

    UserStore::add_user(&mut *user_store, user)
        .await
        .map_err(|_| AuthAPIError::UnexpectedError)?;

    let response = Json(SignupResponse {
        message: "User created successfully!".to_string(),
    });

    Ok((StatusCode::CREATED, response))
}

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    #[serde(rename = "requires2FA")]
    pub requires_2fa: bool,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SignupResponse {
    pub message: String,
}
