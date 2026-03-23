use axum::{
    http::{ Method, StatusCode },
    response::{IntoResponse, Response},
    routing::post,
    serve::Serve,
    Json, Router,
};
use domain::AuthAPIError;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::net::TcpListener;
use tower_http::{ cors::CorsLayer, services::{ ServeDir, ServeFile }};

pub mod app_state;
mod domain;
pub mod routes;
pub mod services;
pub mod utils;

use app_state::{AppState, UserStoreType};
use routes::login::login;
use routes::logout::logout;
use routes::signup::signup;
use routes::verify_2fa::verify_2fa;
use routes::verify_token::verify_token;

// This struct encapsulates our application-related logic.
pub struct Application {
    server: Serve<TcpListener, Router, Router>,
    // address is exposed as a public field
    // so we have access to it in tests.
    pub address: String,
}

impl Application {
    pub async fn build(
        app_state: AppState<UserStoreType, crate::services::hashset_banned_token_store::HashsetBannedTokenStore>,
        address: &str,
    ) -> Result<Self, Box<dyn Error>> {
        // Allow the app service(running on our local machine and in production) to call the auth service
        let allowed_origins = [
            "http://localhost:8000".parse()?,
            "http://147.182.214.35:8000".parse()?,
        ];

        let cors = CorsLayer::new()
            // Allow GET and POST requests
            .allow_methods([Method::GET, Method::POST])
            // Allow cookies to be included in requests
            .allow_credentials(true)
            .allow_origin(allowed_origins);
        
        let assets_dir =
            ServeDir::new("assets").not_found_service(ServeFile::new("assets/index.html"));

        let router = Router::new()
            .route("/login", post(login))
            .route("/logout", post(logout))
            .route("/signup", post(signup))
            .route("/verify-2fa", post(verify_2fa))
            .route("/verify-token", post(verify_token))
            .fallback_service(assets_dir)
            .layer(cors)
            .with_state(app_state);

        let listener = tokio::net::TcpListener::bind(address).await?;
        let address = listener.local_addr()?.to_string();
        let server = axum::serve(listener, router);

        Ok(Self { server, address })
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        println!("listening on {}", &self.address);
        self.server.await
    }
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl IntoResponse for AuthAPIError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthAPIError::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists"),
            AuthAPIError::InvalidCredentials => (StatusCode::BAD_REQUEST, "Invalid credentials"),
            AuthAPIError::AuthenticationFailed => {
                (StatusCode::UNAUTHORIZED, "Incorrect email or password")
            }
            AuthAPIError::MalformedCredentials => (StatusCode::UNPROCESSABLE_ENTITY, "Malformed credentials"),
            AuthAPIError::MissingToken => (StatusCode::BAD_REQUEST, "Missing token"),
            AuthAPIError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthAPIError::UnexpectedError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Unexpected error")
            }
        };
        let body = Json(ErrorResponse {
            error: error_message.to_string(),
        });
        (status, body).into_response()
    }
}
