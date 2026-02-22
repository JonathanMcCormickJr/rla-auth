use axum::{
    response::IntoResponse,
    Router,
    routing::post,
    serve,
};
use reqwest::StatusCode;
use std::error::Error;
use tokio::net::TcpListener;
use tower_http::services::{ ServeDir, ServeFile };

// This struct encapsulates our application-related logic.
pub struct Application {
    server: serve::Serve<TcpListener, Router, Router>,
    // address is exposed as a public field
    // so we have access to it in tests.
    pub address: String,
}

impl Application {
    pub async fn build(address: &str) -> Result<Self, Box<dyn Error>> {
        let assets_dir = ServeDir::new("assets").not_found_service(ServeFile::new("assets/index.html"));

        let router = Router::new().fallback_service(assets_dir)
            // TODO: Add all other routes
            .route("/signup", post(signup)); // Example route;
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

// Example route handler.
// For now we will simply return a 200 (OK) status code.
async fn signup() -> impl IntoResponse {
    StatusCode::OK.into_response()
}

// TODO: Add all other route handlers