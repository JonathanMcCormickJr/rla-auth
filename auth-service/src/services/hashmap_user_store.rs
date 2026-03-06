use std::collections::HashMap;

use crate::domain::{ User, UserStoreError, data_stores::UserStore };

#[derive(Default)]
pub struct HashmapUserStore {
    users: HashMap<String, User>,
}

impl HashmapUserStore {
    pub fn add_user(&mut self, user: User) -> Result<(), UserStoreError> {
        // Return `UserStoreError::UserAlreadyExists` if the user already exists,
        // otherwise insert the user into the hashmap and return `Ok(())`.
        match self.users.get(&user.email) {
            Some(_) => Err(UserStoreError::UserAlreadyExists),
            None => {
                self.users.insert(user.email.clone(), user);
                Ok(())
            }
        }
    }

    pub fn get_user(&self, email: &str) -> Result<User, UserStoreError> {
        match self.users.get(email) {
            Some(user) => Ok(user.clone()),
            None => Err(UserStoreError::UserNotFound),
        }
    }

    pub fn validate_user(&self, email: &str, password: &str) -> Result<(), UserStoreError> {
        match self.users.get(email) {
            Some(user) => {
                if user.password == password {
                    Ok(())
                } else {
                    Err(UserStoreError::InvalidCredentials)
                }
            }
            None => Err(UserStoreError::UserNotFound),
        }
    }
}

#[async_trait::async_trait]
impl UserStore for HashmapUserStore {
    async fn add_user(&mut self, user: User) -> Result<(), UserStoreError> {
        self.add_user(user)
    }

    async fn get_user(&self, email: &str) -> Result<User, UserStoreError> {
        self.get_user(email)
    }

    async fn validate_user(&self, email: &str, password: &str) -> Result<(), UserStoreError> {
        self.validate_user(email, password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_user() {
        let mut user_store = HashmapUserStore::default();

        let user = User::new(
            "test@example.com".to_string(),
            "password123".to_string(),
            true,
        );

        let result = user_store.add_user(user);
        assert_eq!(result, Ok(()));
    }

    #[tokio::test]
    async fn test_get_user() {
        let mut user_store = HashmapUserStore::default();

        let user_email = "test@example.com".to_string();
        let user = User::new(
            user_email.clone(),
            "password123".to_string(),
            true,
        );

        let _ = user_store.add_user(user.clone());

        let result = user_store.get_user(&user_email);
        assert_eq!(result, Ok(user));
    }

    #[tokio::test]
    async fn test_validate_user() {
        let mut user_store = HashmapUserStore::default();

        let user_email = "test@example.com".to_string();
        let user_password = "password123".to_string();
        let user = User::new(
            user_email.clone(),
            user_password.clone(),
            true,
        );
        let _ = user_store.add_user(user);
        let result = user_store.validate_user(&user_email, &user_password);
        assert_eq!(result, Ok(()));
    }
}