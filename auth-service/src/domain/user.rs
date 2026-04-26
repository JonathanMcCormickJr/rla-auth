use secrecy::SecretString;
use uuid::Uuid;

use crate::domain::email::Email;
use crate::domain::password::HashedPassword;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub uuid: Uuid,
    pub email: Email,
    pub password: HashedPassword,
    pub requires_2fa: bool,
}

impl User {
    pub async fn new(
        email: String,
        password: SecretString,
        requires_2fa: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            uuid: Uuid::new_v4(),
            email: Email::parse(SecretString::new(email.into_boxed_str()))
                .map_err(|e| e.to_string())?,
            password: HashedPassword::parse(password)
                .await
                .map_err(|e| e.to_string())?,
            requires_2fa,
        })
    }
}
