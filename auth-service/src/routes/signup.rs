use crate::{
    app_state::{AppState, BannedTokenStoreType, UserStoreType},
    domain::{email::Email, password::Password, AuthAPIError, User},
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
        password: Password::parse(&request.password)
            .map_err(|_| AuthAPIError::InvalidCredentials)?,
        requires_2fa: request.requires_2fa,
    };

    let mut user_store = state.user_store.write().await;

    // Return AuthAPIError::UserAlreadyExists if email exists in user_store.
    if user_store.get_user(&user.email).is_ok() {
        return Err(AuthAPIError::UserAlreadyExists);
    }

    user_store
        .add_user(user)
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
