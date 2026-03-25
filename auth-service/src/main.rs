use auth_service::{
    Application, app_state::{AppState, EmailClientType, UserStoreType}, services::{
        hashmap_2fa_code_store::HashmapTwoFACodeStore,
        hashmap_user_store::HashmapUserStore,
        hashset_banned_token_store::HashsetBannedTokenStore, mock_email_client::MockEmailClient,
    }, utils::constants::prod
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
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
