use auth_service::Application;
use auth_service::app_state::{AppState, BannedTokenStoreType, UserStoreType};
use reqwest::{ Client, cookie::Jar };
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub cookie_jar: Arc<Jar>,
    pub http_client: reqwest::Client,
    pub app_state: AppState<UserStoreType, BannedTokenStoreType>,
}

impl TestApp {
    pub async fn new() -> Self {
        let user_store = auth_service::services::hashmap_user_store::HashmapUserStore::default();
        let banned_token_store = auth_service::services::hashset_banned_token_store::HashsetBannedTokenStore::default();
        let two_fa_code_store = auth_service::services::hashmap_2fa_code_store::HashmapTwoFACodeStore::default();
        let app_state = auth_service::app_state::AppState::new(std::sync::Arc::new(
            tokio::sync::RwLock::new(user_store),
        )
            as auth_service::app_state::UserStoreType, banned_token_store, two_fa_code_store, std::sync::Arc::new(auth_service::services::mock_email_client::MockEmailClient) as auth_service::app_state::EmailClientType);


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

    pub async fn post_verify_2fa(&self) -> reqwest::Response {
        self.http_client
            .post(&format!("{}/verify-2fa", &self.address))
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
}

pub fn get_random_email() -> String {
    let random_uuid = Uuid::new_v4();
    format!("{}@example.com", random_uuid)
}
