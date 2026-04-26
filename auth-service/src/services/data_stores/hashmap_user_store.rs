use std::collections::HashMap;

use secrecy::SecretString;

use crate::domain::{data_stores::UserStore, email::Email, User, UserStoreError};

#[derive(Default)]
pub struct HashmapUserStore {
    users: HashMap<Email, User>,
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

    pub fn get_user(&self, email: &Email) -> Result<User, UserStoreError> {
        match self.users.get(email) {
            Some(user) => Ok(user.clone()),
            None => Err(UserStoreError::UserNotFound),
        }
    }

    pub async fn validate_user(
        &self,
        email: &Email,
        password: &SecretString,
    ) -> Result<(), UserStoreError> {
        match self.users.get(email) {
            Some(user) => {
                if user.password.verify_raw_password(password).await.is_ok() {
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

    async fn get_user(&self, email: &Email) -> Result<User, UserStoreError> {
        self.get_user(email)
    }

    async fn validate_user(
        &self,
        email: &Email,
        raw_password: &SecretString,
    ) -> Result<(), UserStoreError> {
        let user: &User = self.users.get(email).ok_or(UserStoreError::UserNotFound)?;

        user.password
            .verify_raw_password(raw_password)
            .await
            .map_err(|_| UserStoreError::InvalidCredentials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(s: &str) -> SecretString {
        SecretString::new(s.to_owned().into_boxed_str())
    }

    #[tokio::test]
    async fn test_add_user() {
        let mut user_store = HashmapUserStore::default();

        let user = User::new(
            "test@example.com".to_string(),
            secret("Password123!"),
            true,
        )
        .await
        .unwrap();

        let result = user_store.add_user(user);
        assert_eq!(result, Ok(()));
    }

    #[tokio::test]
    async fn test_get_user() {
        let mut user_store = HashmapUserStore::default();

        let user_email = "test@example.com".to_string();
        let user = User::new(user_email.clone(), secret("Password123!"), true)
            .await
            .unwrap();

        let _ = user_store.add_user(user.clone());

        let result = user_store.get_user(&Email::parse(secret(&user_email)).unwrap());
        assert_eq!(result, Ok(user));
    }

    #[tokio::test]
    async fn test_validate_user() {
        let mut user_store = HashmapUserStore::default();

        let user_email = "test@example.com".to_string();
        let user_password = "Password123!";
        let user = User::new(user_email.clone(), secret(user_password), true)
            .await
            .unwrap();
        let _ = user_store.add_user(user);
        let result = user_store
            .validate_user(
                &Email::parse(secret(&user_email)).unwrap(),
                &secret(user_password),
            )
            .await;
        assert_eq!(result, Ok(()));
    }
}
