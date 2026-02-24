use axum::{routing::post, serve, Router};
use std::error::Error;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};

pub mod routes;

use routes::signup::signup;

// This struct encapsulates our application-related logic.
pub struct Application {
    server: serve::Serve<TcpListener, Router, Router>,
    // address is exposed as a public field
    // so we have access to it in tests.
    pub address: String,
}

impl Application {
    pub async fn build(address: &str) -> Result<Self, Box<dyn Error>> {
        let assets_dir =
            ServeDir::new("assets").not_found_service(ServeFile::new("assets/index.html"));

        let router = Router::new()
            .fallback_service(assets_dir)
            .route("/signup", post(signup))
            .route("/login", post(routes::login::login))
            .route("/logout", post(routes::logout::logout))
            .route("/verify-2fa", post(routes::verify_2fa::verify_2fa))
            .route("/verify-token", post(routes::verify_token::verify_token));
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
