use std::collections::HashMap;

use crate::domain::{
    data_stores::UserStore, email::Email, password::Password, User, UserStoreError,
};

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

    pub fn validate_user(&self, email: &Email, password: &Password) -> Result<(), UserStoreError> {
        match self.users.get(email) {
            Some(user) => {
                if user.password == *password {
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
        password: &Password,
    ) -> Result<(), UserStoreError> {
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
            "Password123!".to_string(),
            true,
        );

        let result = user_store.add_user(user);
        assert_eq!(result, Ok(()));
    }

    #[tokio::test]
    async fn test_get_user() {
        let mut user_store = HashmapUserStore::default();

        let user_email = "test@example.com".to_string();
        let user = User::new(user_email.clone(), "Password123!".to_string(), true);

        let _ = user_store.add_user(user.clone());

        let result = user_store.get_user(&Email::parse(&user_email).unwrap());
        assert_eq!(result, Ok(user));
    }

    #[tokio::test]
    async fn test_validate_user() {
        let mut user_store = HashmapUserStore::default();

        let user_email = "test@example.com".to_string();
        let user_password = "Password123!".to_string();
        let user = User::new(user_email.clone(), user_password.clone(), true);
        let _ = user_store.add_user(user);
        let result = user_store.validate_user(
            &Email::parse(&user_email).unwrap(),
            &Password::parse(&user_password).unwrap(),
        );
        assert_eq!(result, Ok(()));
    }
}
