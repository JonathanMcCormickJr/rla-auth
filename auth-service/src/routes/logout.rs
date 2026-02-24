use axum::response::IntoResponse;
use reqwest::StatusCode;

// For now we will simply return a 200 (OK) status code.
pub async fn logout() -> impl IntoResponse {
    StatusCode::OK.into_response()
}
