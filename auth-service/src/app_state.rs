use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{domain::data_stores::BannedTokenStore, services::hashmap_user_store::HashmapUserStore};

// Using a type alias to improve readability!
pub type UserStoreType = Arc<RwLock<HashmapUserStore>>;
pub type BannedTokenStoreType = crate::services::hashset_banned_token_store::HashsetBannedTokenStore;

#[derive(Clone)]
pub struct AppState<T: Send + Sync, Btt: BannedTokenStore> {
    pub user_store: T,
    pub banned_token_store: Arc<RwLock<Btt>>,
}

impl<T: Send + Sync, Btt: BannedTokenStore + Default> AppState<T, Btt> {
    pub fn new(user_store: T, banned_token_store: Btt) -> Self {
        Self { user_store, banned_token_store: Arc::new(RwLock::new(banned_token_store)) }
    }
}