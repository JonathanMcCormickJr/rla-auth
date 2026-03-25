use auth_service::{
    app_state::{AppState, UserStoreType},
    services::{
        hashmap_2fa_code_store::HashmapTwoFACodeStore,
        hashmap_user_store::HashmapUserStore,
        hashset_banned_token_store::HashsetBannedTokenStore,
    },
    utils::constants::prod,
    Application,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let user_store = HashmapUserStore::default();
    let banned_token_store = HashsetBannedTokenStore::default();
    let two_fa_code_store = HashmapTwoFACodeStore::default();

    let app_state = AppState::new(
        Arc::new(RwLock::new(user_store)) as UserStoreType,
        banned_token_store,
        two_fa_code_store,
    );

    let app = Application::build(app_state, prod::APP_ADDRESS)
        .await
        .expect("Failed to build app");

    app.run().await.expect("Failed to run app");
}
