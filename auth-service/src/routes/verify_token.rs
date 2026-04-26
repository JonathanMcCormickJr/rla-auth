use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;

use crate::{
    app_state::{AppState, BannedTokenStoreType, UserStoreType},
    domain::AuthAPIError,
    utils::auth::validate_token,
};

#[tracing::instrument(name = "Verify Token", skip_all)]
pub async fn verify_token(
    State(state): State<AppState<UserStoreType, BannedTokenStoreType>>,
    Json(request): Json<VerifyTokenRequest>,
) -> Result<impl IntoResponse, AuthAPIError> {
    let token = request.token.trim();

    if token.is_empty() {
        return Err(AuthAPIError::MissingToken);
    }

    // Reject obviously malformed JWTs before attempting signature validation.
    let mut parts = token.split('.');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(header), Some(payload), Some(signature), None)
            if !header.is_empty() && !payload.is_empty() && !signature.is_empty() => {}
        _ => return Err(AuthAPIError::InvalidToken),
    }

    let claims = validate_token(token, &state.banned_token_store)
        .await
        .map_err(|_| AuthAPIError::InvalidToken)?;

    if claims.sub.trim().is_empty() {
        return Err(AuthAPIError::InvalidToken);
    }

    Ok(StatusCode::OK.into_response())
}

#[derive(Deserialize)]
pub struct VerifyTokenRequest {
    pub token: String,
}
