use auth_service::{
    Application, app_state::{AppState, EmailClientType, UserStoreType}, get_postgres_pool, services::data_stores::{
        hashmap_2fa_code_store::HashmapTwoFACodeStore, hashmap_user_store::HashmapUserStore,
        hashset_banned_token_store::HashsetBannedTokenStore, mock_email_client::MockEmailClient,
    }, utils::constants::{
        DATABASE_URL, JWT_SECRET, env::{DATABASE_URL_ENV_VAR, JWT_SECRET_ENV_VAR}, prod
    }
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    // We will use this PostgreSQL pool in the next task! 
    let pg_pool = configure_postgresql().await;

    let user_store = HashmapUserStore::default();
    let banned_token_store = HashsetBannedTokenStore::default();
    let two_fa_code_store = HashmapTwoFACodeStore::default();
    let email_client = Arc::new(MockEmailClient) as EmailClientType; // TODO: Replace with real email client in production!
    let app_state = AppState::new(
        Arc::new(RwLock::new(user_store)) as UserStoreType,
        banned_token_store,
        two_fa_code_store,
        email_client,
    );

    let app = Application::build(app_state, prod::APP_ADDRESS)
        .await
        .expect("Failed to build app");

    app.run().await.expect("Failed to run app");
}

async fn configure_postgresql() -> PgPool {
    // Create a new database connection pool
    let pg_pool = get_postgres_pool(&DATABASE_URL)
        .await
        .expect("Failed to create Postgres connection pool!");

    // Run database migrations against our test database! 
    sqlx::migrate!()
        .run(&pg_pool)
        .await
        .expect("Failed to run migrations");

    pg_pool
}