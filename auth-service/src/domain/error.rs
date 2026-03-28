pub enum AuthAPIError {
    UserAlreadyExists,    // Corresponds to a 409 Conflict HTTP status code
    InvalidCredentials,   // Corresponds to a 400 Bad Request HTTP status code
    AuthenticationFailed, // Corresponds to a 401 Unauthorized HTTP status code
    MalformedCredentials, // Corresponds to a 422 Unprocessable Entity HTTP status code
    MissingToken,
    InvalidToken,
    UnexpectedError, // Corresponds to a 500 Internal Server Error HTTP status code
}
