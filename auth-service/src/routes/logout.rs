use axum::{http::StatusCode, response::IntoResponse};
use axum::extract::State;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;

use crate::{
    app_state::{AppState, BannedTokenStoreType, UserStoreType},
    domain::data_stores::BannedTokenStore,
    domain::AuthAPIError,
    utils::{auth::validate_token, constants::JWT_COOKIE_NAME},
};

pub async fn logout(
    State(state): State<AppState<UserStoreType, BannedTokenStoreType>>,
    jar: CookieJar,
) -> (CookieJar, Result<impl IntoResponse, AuthAPIError>) {
    // Retrieve JWT cookie from the `CookieJar`
    // Return AuthAPIError::MissingToken is the cookie is not found
    let cookie = match jar.get(JWT_COOKIE_NAME) {
        Some(cookie) => cookie,
        None => return (jar, Err(AuthAPIError::MissingToken)),
    };

    let token = cookie.value().to_owned();

    if validate_token(&token, &state.banned_token_store).await.is_err() {
        return (jar, Err(AuthAPIError::InvalidToken));
    }

    let mut banned_store = state.banned_token_store.write().await;
    if banned_store.add_token(token).await.is_err() {
        return (jar, Err(AuthAPIError::UnexpectedError));
    }

    // Remove the JWT cookie from the CookieJar.
    // remove() requires an owned cookie, not the borrowed reference from jar.get().
    let removal_cookie = Cookie::build((JWT_COOKIE_NAME, "")).path("/").build();
    let jar = jar.remove(removal_cookie);
    (jar, Ok(StatusCode::OK.into_response()))
}