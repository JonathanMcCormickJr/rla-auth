use auth_service::Application;

#[tokio::main]
async fn main() {
    let assets_dir = tower_http::services::ServeDir::new("assets");
    let app = Application::build("0.0.0.0:3000", assets_dir)
        .await
        .expect("Failed to build app");

    app.run().await.expect("Failed to run app");
}