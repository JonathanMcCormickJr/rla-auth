use axum::{
    Router,
    serve,
};
use std::error::Error;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

// This struct encapsulates our application-related logic.
pub struct Application {
    server: serve::Serve<TcpListener, Router, Router>,
    // address is exposed as a public field
    // so we have access to it in tests.
    pub address: String,
}

impl Application {
    pub async fn build(address: &str, assets_dir: ServeDir) -> Result<Self, Box<dyn Error>> {
        let router = Router::new().fallback_service(assets_dir);
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