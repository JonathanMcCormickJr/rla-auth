use auth_service::app_state::{AppState, BannedTokenStoreType, EmailClientType, UserStoreType};
use auth_service::services::data_stores::{
    mock_email_client::MockEmailClient, postgres_user_store::PostgresUserStore,
    redis_banned_token_store::RedisBannedTokenStore,
    redis_two_fa_code_store::RedisTwoFACodeStore,
};
use auth_service::utils::constants::{DATABASE_URL, REDIS_HOST_NAME};
use auth_service::utils::docker_test_env::DockerEnv;
use auth_service::{get_postgres_pool, get_redis_client, Application};
use secrecy::{ExposeSecret, SecretString};
use reqwest::{cookie::Jar, Client};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub cookie_jar: Arc<Jar>,
    pub http_client: reqwest::Client,
    pub app_state: AppState<UserStoreType, BannedTokenStoreType>,
    pub test_db_name: String,
    pub clean_up_called: bool,
    _docker_env: DockerEnv,
}

impl TestApp {
    pub async fn new() -> Self {
        let docker_env = DockerEnv::ensure();
        let (pg_pool, test_db_name) = configure_postgresql().await;

        let user_store = Arc::new(RwLock::new(PostgresUserStore::new(pg_pool))) as UserStoreType;
        let banned_token_store =
            RedisBannedTokenStore::new(Arc::new(RwLock::new(redis_connection())));
        let two_fa_code_store =
            RedisTwoFACodeStore::new(Arc::new(RwLock::new(redis_connection())));
        let app_state = AppState::new(
            user_store,
            banned_token_store,
            two_fa_code_store,
            Arc::new(MockEmailClient) as EmailClientType,
        );

        let app = Application::build(app_state.clone(), "0.0.0.0:0")
            .await
            .expect("Failed to build app");

        let address = format!("http://{}", app.address.clone());

        // Run the auth service in a separate async task
        // to avoid blocking the main test thread.
        #[allow(clippy::let_underscore_future)]
        let _ = tokio::spawn(app.run());

        let cookie_jar = Arc::new(Jar::default());

        let http_client = Client::builder()
            .cookie_provider(Arc::clone(&cookie_jar))
            .build()
            .expect("Failed to build HTTP client");

        // Create new `TestApp` instance and return it
        Self {
            address,
            cookie_jar,
            http_client,
            app_state,
            test_db_name,
            clean_up_called: false,
            _docker_env: docker_env,
        }
    }

    pub async fn get_root(&self) -> reqwest::Response {
        self.http_client
            .get(&format!("{}/", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_signup(&self) -> reqwest::Response {
        let body = json!({
            "email": get_random_email(),
            "password": "Password123!",
            "requires2FA": true
        });

        self.post_signup_with_body(&body).await
    }

    pub async fn post_signup_with_body(&self, body: &serde_json::Value) -> reqwest::Response {
        self.http_client
            .post(&format!("{}/signup", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.http_client
            .post(&format!("{}/login", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.http_client
            .post(&format!("{}/logout", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_verify_2fa<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.http_client
            .post(format!("{}/verify-2fa", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_verify_token<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.http_client
            .post(&format!("{}/verify-token", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn clean_up(&mut self) {
        delete_database(&self.test_db_name).await;
        self.clean_up_called = true;
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        if !self.clean_up_called {
            panic!("TestApp was dropped without calling clean_up()");
        }
    }
}

pub fn get_random_email() -> String {
    let random_uuid = Uuid::new_v4();
    format!("{}@example.com", random_uuid)
}

fn redis_connection() -> redis::Connection {
    get_redis_client(REDIS_HOST_NAME.to_owned())
        .expect("Failed to open Redis client")
        .get_connection()
        .expect("Failed to get Redis connection")
}

async fn configure_postgresql() -> (PgPool, String) {
    let postgresql_conn_url = DATABASE_URL.expose_secret().to_owned();

    // We are creating a new database for each test case, and we need to ensure each database has a unique name!
    let db_name = Uuid::new_v4().to_string();

    configure_database(&postgresql_conn_url, &db_name).await;

    let postgresql_conn_url_with_db =
        SecretString::new(format!("{}/{}", postgresql_conn_url, db_name).into_boxed_str());

    let pool = get_postgres_pool(&postgresql_conn_url_with_db)
        .await
        .expect("Failed to create Postgres connection pool!");

    (pool, db_name)
}

async fn configure_database(db_conn_string: &str, db_name: &str) {
    // Create database connection
    let connection = PgPoolOptions::new()
        .connect(db_conn_string)
        .await
        .expect("Failed to create Postgres connection pool.");

    // Create a new database
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_name).as_str())
        .await
        .expect("Failed to create database.");

    // Connect to new database
    let db_conn_string = format!("{}/{}", db_conn_string, db_name);

    let connection = PgPoolOptions::new()
        .connect(&db_conn_string)
        .await
        .expect("Failed to create Postgres connection pool.");

    // Run migrations against new database
    sqlx::migrate!()
        .run(&connection)
        .await
        .expect("Failed to migrate the database");
}

async fn delete_database(db_name: &str) {
    let postgresql_conn_url: String = DATABASE_URL.expose_secret().to_owned();

    let connection_options = PgConnectOptions
        ::from_str(&postgresql_conn_url)
        .expect("Failed to parse PostgreSQL connection string");

    let mut connection = PgConnection::connect_with(&connection_options)
        .await
        .expect("Failed to connect to Postgres");

    // Kill any active connections to the database
    connection
        .execute(
            format!(
                r#"
                SELECT pg_terminate_backend(pg_stat_activity.pid)
                FROM pg_stat_activity
                WHERE pg_stat_activity.datname = '{}'
                  AND pid <> pg_backend_pid();
        "#,
                db_name
            )
            .as_str(),
        )
        .await
        .expect("Failed to drop the database.");

    // Drop the database
    connection
        .execute(format!(r#"DROP DATABASE "{}";"#, db_name).as_str())
        .await
        .expect("Failed to drop the database.");
}
