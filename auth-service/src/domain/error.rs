use color_eyre::eyre::Report;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthAPIError {
    #[error("User already exists")]
    UserAlreadyExists, // Corresponds to a 409 Conflict HTTP status code
    #[error("Invalid Credentials")]
    InvalidCredentials, // Corresponds to a 400 Bad Request HTTP status code
    #[error("Athentication failed")]
    AuthenticationFailed, // Corresponds to a 401 Unauthorized HTTP status code
    #[error("Malfformed credentials")]
    MalformedCredentials, // Corresponds to a 422 Unprocessable Entity HTTP status code
    #[error("Missing token")]
    MissingToken,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Unexpected error")]
    UnexpectedError(#[source] Report), // Corresponds to a 500 Internal Server Error HTTP status code
}
