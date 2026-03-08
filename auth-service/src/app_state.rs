use std::sync::Arc;
use tokio::sync::RwLock;

use crate::services::hashmap_user_store::HashmapUserStore;

// Using a type alias to improve readability!
pub type UserStoreType = Arc<RwLock<HashmapUserStore>>;

#[derive(Clone)]
pub struct AppState<T: Send + Sync> {
    pub user_store: T,
}

impl<T: Send + Sync> AppState<T> {
    pub fn new(user_store: T) -> Self {
        Self { user_store }
    }
}
