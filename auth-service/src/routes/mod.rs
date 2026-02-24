use axum::response::IntoResponse;
use reqwest::StatusCode;

pub mod login;
pub mod logout;
pub mod signup;
pub mod verify_2fa;
pub mod verify_token;

// re-export items from sub-modules
pub use login::*;
pub use logout::*;
pub use signup::*;
pub use verify_2fa::*;
pub use verify_token::*;

// TODO: Add all other route handlers within respective modules (login, logout, verify-2fa, and verify-token)
