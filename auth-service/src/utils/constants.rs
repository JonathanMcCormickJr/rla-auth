use dotenvy::dotenv;
use lazy_static::lazy_static;
use secrecy::SecretString;
use std::env as std_env;

// Define a lazily evaluated static. lazy_static is needed because std_env::var is not a const function.
lazy_static! {
    pub static ref JWT_SECRET: SecretString = set_token();
    pub static ref DATABASE_URL: SecretString = set_database_url();
    pub static ref REDIS_HOST_NAME: String = set_redis_host();
    pub static ref REDIS_PASSWORD: Option<SecretString> = set_redis_password();
    pub static ref RESEND_API_KEY: SecretString = set_resend_auth_token();
}

fn set_token() -> SecretString {
    dotenv().ok(); // Load environment variables
    let secret = std_env::var(env::JWT_SECRET_ENV_VAR).expect("JWT_SECRET must be set.");
    if secret.is_empty() {
        panic!("JWT_SECRET must not be empty.");
    }
    SecretString::new(secret.into_boxed_str())
}

fn set_database_url() -> SecretString {
    dotenv().ok(); // Load environment variables
    let url = std_env::var(env::DATABASE_URL_ENV_VAR).expect("DATABASE_URL must be set.");
    if url.is_empty() {
        panic!("DATABASE_URL must not be empty.");
    }
    SecretString::new(url.into_boxed_str())
}

fn set_redis_host() -> String {
    dotenv().ok();
    std_env::var(env::REDIS_HOST_NAME_ENV_VAR).unwrap_or(DEFAULT_REDIS_HOSTNAME.to_owned())
}

pub const JWT_COOKIE_NAME: &str = "jwt";
pub const DEFAULT_REDIS_HOSTNAME: &str = "127.0.0.1";

fn set_redis_password() -> Option<SecretString> {
    dotenv().ok();
    match std_env::var(env::REDIS_PASSWORD_ENV_VAR) {
        Ok(s) if !s.is_empty() => Some(SecretString::new(s.into_boxed_str())),
        _ => None,
    }
}

fn set_resend_auth_token() -> SecretString {
    dotenv().ok();
    SecretString::new(
        std_env::var(env::RESEND_AUTH_TOKEN_ENV_VAR)
            .expect("RESEND_API_KEY must be set.")
            .into_boxed_str(),
    )
}
pub mod env {
    pub const JWT_SECRET_ENV_VAR: &str = "JWT_SECRET";
    pub const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
    pub const REDIS_HOST_NAME_ENV_VAR: &str = "REDIS_HOST_NAME";
    pub const REDIS_PASSWORD_ENV_VAR: &str = "REDIS_PASSWORD";
    pub const RESEND_AUTH_TOKEN_ENV_VAR: &str = "RESEND_API_KEY";
}

pub mod prod {
    pub const APP_ADDRESS: &str = "0.0.0.0:3000";
    pub mod email_client {
        use std::time::Duration;

        pub const BASE_URL: &str = "https://api.resend.com";
        pub const SENDER_RESEND: &str = "onboarding@resend.dev";
        pub const TIMEOUT: Duration = std::time::Duration::from_secs(10);
    }
}

pub mod test {
    pub const APP_ADDRESS: &str = "127.0.0.1:0";
    pub mod email_client {
        use std::time::Duration;

        pub const SENDER: &str = "test@email.com";
        pub const TIMEOUT: Duration = std::time::Duration::from_millis(200);
    }
}
