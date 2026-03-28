use super::{email::Email, password::Password, User};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait UserStore {
    async fn add_user(&mut self, user: User) -> Result<(), UserStoreError>;
    async fn get_user(&self, email: &Email) -> Result<User, UserStoreError>;
    async fn validate_user(&self, email: &Email, password: &Password)
        -> Result<(), UserStoreError>;
}

#[derive(Debug, PartialEq)]
pub enum UserStoreError {
    UserAlreadyExists,
    UserNotFound,
    InvalidCredentials,
    MalformedCredentials,
    UnexpectedError,
}

#[async_trait::async_trait]
pub trait BannedTokenStore {
    async fn add_token(&mut self, token: String) -> Result<(), BannedTokenStoreError>;
    async fn is_token_banned(&self, token: &String) -> Result<bool, BannedTokenStoreError>;
}

#[derive(Debug, PartialEq)]
pub enum BannedTokenStoreError {
    TokenAlreadyBanned,
    TokenNotFound,
    UnexpectedError,
}

// This trait represents the interface all concrete 2FA code stores should implement
#[async_trait::async_trait]
pub trait TwoFACodeStore {
    async fn add_code(
        &mut self,
        email: Email,
        login_attempt_id: LoginAttemptId,
        code: TwoFACode,
    ) -> Result<(), TwoFACodeStoreError>;
    async fn remove_code(&mut self, email: &Email) -> Result<(), TwoFACodeStoreError>;
    async fn get_code(
        &self,
        email: &Email,
    ) -> Result<(LoginAttemptId, TwoFACode), TwoFACodeStoreError>;
}

#[derive(Debug, PartialEq)]
pub enum TwoFACodeStoreError {
    LoginAttemptIdNotFound,
    UnexpectedError,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoginAttemptId(String);

impl LoginAttemptId {
    pub fn parse(id: String) -> Result<Self, String> {
        // Use the `parse_str` function from the `uuid` crate to ensure `id` is a valid UUID
        Uuid::parse_str(&id)
            .map(|_| Self(id))
            .map_err(|error| error.to_string())
    }
}

impl Default for LoginAttemptId {
    fn default() -> Self {
        // Use the `uuid` crate to generate a random version 4 UUID
        Self(Uuid::new_v4().to_string())
    }
}

impl AsRef<str> for LoginAttemptId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TwoFACode(String);

impl TwoFACode {
    pub fn parse(_code: String) -> Result<Self, String> {
        // Ensure `code` is a valid 6-digit code
        if _code.len() == 6 && _code.chars().all(|c| c.is_digit(10)) {
            Ok(Self(_code))
        } else {
            Err("2FA code must be a 6-digit number".to_string())
        }
    }
}

impl Default for TwoFACode {
    fn default() -> Self {
        // Use the `rand` crate to generate a random 2FA code.
        // The code should be 6 digits (ex: 834629)
        let random_bytes: [u8; 3] = rand::random();
        let random_number =
            u32::from_be_bytes([0, random_bytes[0], random_bytes[1], random_bytes[2]]) % 1_000_000;
        Self(format!("{:06}", random_number))
    }
}

impl AsRef<str> for TwoFACode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
