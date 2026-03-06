pub enum AuthAPIError {
    UserAlreadyExists, // Corresponds to a 409 Conflict HTTP status code
    InvalidCredentials, // Corresponds to a 400 Bad Request HTTP status code
    UnexpectedError, // Corresponds to a 500 Internal Server Error HTTP status code
}