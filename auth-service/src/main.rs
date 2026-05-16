use auth_service::{
    Application,
    app_state::{AppState, UserStoreType},
    domain::Email,
    get_postgres_pool, get_redis_client,
    services::{
        data_stores::{
            postgres_user_store::PostgresUserStore,
            redis_banned_token_store::RedisBannedTokenStore,
            redis_two_fa_code_store::RedisTwoFACodeStore,
        },
        resend_email_client::ResendEmailClient,
    },
    utils::{
        constants::{DATABASE_URL, REDIS_HOST_NAME, RESEND_API_KEY, prod},
        tracing::init_tracing,
    },
};
use reqwest::Client;
use secrecy::SecretString;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    color_eyre::install().expect("Failed to install color_eyre");
    init_tracing().expect("Failed to initialize tracing");
    let pg_pool = configure_postgresql().await;

    let user_store = PostgresUserStore::new(pg_pool);
    let banned_token_store =
        RedisBannedTokenStore::new(Arc::new(RwLock::new(configure_redis())));
    let two_fa_code_store =
        RedisTwoFACodeStore::new(Arc::new(RwLock::new(configure_redis())));
    let email_client = Arc::new(configure_resend_email_client());
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

fn configure_redis() -> redis::Connection {
    get_redis_client(REDIS_HOST_NAME.to_owned())
        .expect("Failed to get Redis client")
        .get_connection()
        .expect("Failed to get Redis connection")
}

fn configure_resend_email_client() -> ResendEmailClient {
    let http_client = Client::builder()
        .timeout(prod::email_client::TIMEOUT)
        .build()
        .expect("Failed to build HTTP client");

    ResendEmailClient::new(
        prod::email_client::BASE_URL.to_owned(),
        Email::parse(SecretString::new(
            prod::email_client::SENDER_RESEND.to_owned().into_boxed_str(),
        ))
        .unwrap(),
        RESEND_API_KEY.to_owned(),
        http_client,
    )
}
