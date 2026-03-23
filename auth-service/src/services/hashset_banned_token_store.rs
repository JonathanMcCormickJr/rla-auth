use crate::domain::data_stores::BannedTokenStore;

#[derive(Clone)]
pub struct HashsetBannedTokenStore {
    banned_tokens: std::collections::HashSet<String>,
}

impl Default for HashsetBannedTokenStore {
    fn default() -> Self {
        Self {
            banned_tokens: std::collections::HashSet::new(),
        }
    }
}

#[async_trait::async_trait]
impl BannedTokenStore for HashsetBannedTokenStore {
    async fn add_token(
        &mut self,
        token: String,
    ) -> Result<(), crate::domain::data_stores::BannedTokenStoreError> {
        self.add_banned_token(token)
            .map_err(|_| crate::domain::data_stores::BannedTokenStoreError::UnexpectedError)
    }

    async fn is_token_banned(
        &self,
        token: &String,
    ) -> Result<bool, crate::domain::data_stores::BannedTokenStoreError> {
        HashsetBannedTokenStore::is_token_banned(self, token.as_str())
            .map_err(|_| crate::domain::data_stores::BannedTokenStoreError::UnexpectedError)
    }
}

impl HashsetBannedTokenStore {
    pub fn add_banned_token(&mut self, token: String) -> Result<(), ()> {
        self.banned_tokens.insert(token);
        Ok(())
    }

    pub fn is_token_banned(&self, token: &str) -> Result<bool, ()> {
        Ok(self.banned_tokens.contains(token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_banned_token() {
        let mut token_store = HashsetBannedTokenStore::default();
        let token = "test_token".to_string();
        let result = token_store.add_banned_token(token.clone());
        assert_eq!(result, Ok(()));
        assert!(token_store.is_token_banned(&token).unwrap());
    }

    #[tokio::test]
    async fn test_is_token_banned() {
        let mut token_store = HashsetBannedTokenStore::default();
        let token = "test_token".to_string();
        assert!(!token_store.is_token_banned(&token).unwrap());
        let _ = token_store.add_banned_token(token.clone());
        assert!(token_store.is_token_banned(&token).unwrap());
    }
}