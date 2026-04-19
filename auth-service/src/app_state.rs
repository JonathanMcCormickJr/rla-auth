use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    domain::data_stores::BannedTokenStore, services::data_stores::postgres_user_store::PostgresUserStore,
};

pub type UserStoreType = Arc<RwLock<PostgresUserStore>>;
pub type BannedTokenStoreType =
    crate::services::data_stores::redis_banned_token_store::RedisBannedTokenStore;
pub type TwoFACodeStoreType = crate::services::data_stores::redis_two_fa_code_store::RedisTwoFACodeStore;
pub type TwoFACodeStoreHandle = Arc<RwLock<TwoFACodeStoreType>>;
pub type EmailClientType = Arc<dyn crate::domain::EmailClient + Send + Sync>; // New!

#[derive(Clone)]
pub struct AppState<T: Send + Sync, Btt: BannedTokenStore> {
    pub user_store: T,
    pub banned_token_store: Arc<RwLock<Btt>>,
    pub two_fa_code_store: TwoFACodeStoreHandle,
    pub email_client: EmailClientType,
}

impl<T: Send + Sync, Btt: BannedTokenStore> AppState<T, Btt> {
    pub fn new(
        user_store: T,
        banned_token_store: Btt,
        two_fa_code_store: TwoFACodeStoreType,
        email_client: EmailClientType,
    ) -> Self {
        Self {
            user_store,
            banned_token_store: Arc::new(RwLock::new(banned_token_store)),
            two_fa_code_store: Arc::new(RwLock::new(two_fa_code_store)),
            email_client,
        }
    }
}
