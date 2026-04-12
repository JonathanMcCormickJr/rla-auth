use uuid::Uuid;

use crate::domain::email::Email;
use crate::domain::password::HashedPassword;

// The User struct should contain 3 fields. email, which is a String;
// password, which is also a String; and requires_2fa, which is a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub uuid: Uuid,
    pub email: Email,
    pub password: HashedPassword,
    pub requires_2fa: bool,
}

impl User {
    pub async fn new(email: String, password: String, requires_2fa: bool) -> Result<Self, String> {
        Ok(Self {
            uuid: Uuid::new_v4(),
            email: Email::parse(&email).map_err(|e| e.to_string())?,
            password: HashedPassword::parse(password).await?,
            requires_2fa,
        })
    }
}
