use auth_service::Application;

#[tokio::main]
async fn main() {
    let user_store = auth_service::services::hashmap_user_store::HashmapUserStore::default();
    let app_state = auth_service::app_state::AppState::new(
        std::sync::Arc::new(tokio::sync::RwLock::new(
            user_store,
        )) as auth_service::app_state::UserStoreType,
    );

    let app = Application::build(app_state, "0.0.0.0:3000")
        .await
        .expect("Failed to build app");

    app.run().await.expect("Failed to run app");
}