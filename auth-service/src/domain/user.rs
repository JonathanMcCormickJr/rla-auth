use uuid::Uuid;

// The User struct should contain 3 fields. email, which is a String; 
// password, which is also a String; and requires_2fa, which is a boolean. 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub uuid: Uuid,
    pub email: String,
    pub password: String, // TODO: in a real application, we would never store passwords in plain text. We would hash them first.
    pub requires_2fa: bool,
}

impl User {
    pub fn new(email: String, password: String, requires_2fa: bool) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            email,
            password,
            requires_2fa,
        }
    }
}