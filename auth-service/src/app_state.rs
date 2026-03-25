use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{domain::data_stores::BannedTokenStore, services::hashmap_user_store::HashmapUserStore};

// Using a type alias to improve readability!
pub type UserStoreType = Arc<RwLock<HashmapUserStore>>;
pub type BannedTokenStoreType = crate::services::hashset_banned_token_store::HashsetBannedTokenStore;
pub type TwoFACodeStoreType = crate::services::hashmap_2fa_code_store::HashmapTwoFACodeStore;
pub type TwoFACodeStoreHandle = Arc<RwLock<TwoFACodeStoreType>>;
pub type EmailClientType = Arc<dyn crate::domain::EmailClient + Send + Sync>; // New!

#[derive(Clone)]
pub struct AppState<T: Send + Sync, Btt: BannedTokenStore> {
    pub user_store: T,
    pub banned_token_store: Arc<RwLock<Btt>>,
    pub two_fa_code_store: TwoFACodeStoreHandle,
    pub email_client: EmailClientType,
}

impl<T: Send + Sync, Btt: BannedTokenStore + Default> AppState<T, Btt> {
    pub fn new(user_store: T, banned_token_store: Btt, two_fa_code_store: TwoFACodeStoreType, email_client: EmailClientType) -> Self {
        Self {
            user_store,
            banned_token_store: Arc::new(RwLock::new(banned_token_store)),
            two_fa_code_store: Arc::new(RwLock::new(two_fa_code_store)),
            email_client,
        }
    }
}