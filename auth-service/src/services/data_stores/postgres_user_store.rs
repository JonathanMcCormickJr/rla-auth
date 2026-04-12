use sqlx::{PgPool, Row};

use crate::domain::{
    data_stores::{UserStore, UserStoreError},
    Email, HashedPassword, User,
};

pub struct PostgresUserStore {
    pool: PgPool,
}

impl PostgresUserStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserStore for PostgresUserStore {
    async fn add_user(&mut self, user: User) -> Result<(), UserStoreError> {
        sqlx::query(
            "INSERT INTO users (email, password_hash, requires_2fa) VALUES ($1, $2, $3)",
        )
        .bind(user.email.as_ref())
        .bind(user.password.as_ref())
        .bind(user.requires_2fa)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                UserStoreError::UserAlreadyExists
            }
            _ => UserStoreError::UnexpectedError,
        })?;
        Ok(())
    }

    async fn get_user(&self, email: &Email) -> Result<User, UserStoreError> {
        let row = sqlx::query(
            "SELECT email, password_hash, requires_2fa FROM users WHERE email = $1",
        )
        .bind(email.as_ref())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| UserStoreError::UnexpectedError)?
        .ok_or(UserStoreError::UserNotFound)?;

        let db_email: String = row.get("email");
        let password_hash: String = row.get("password_hash");
        let requires_2fa: bool = row.get("requires_2fa");

        let email =
            Email::parse(&db_email).map_err(|_| UserStoreError::MalformedCredentials)?;
        let password = HashedPassword::parse_password_hash(password_hash)
            .map_err(|_| UserStoreError::UnexpectedError)?;

        Ok(User {
            uuid: uuid::Uuid::new_v4(),
            email,
            password,
            requires_2fa,
        })
    }

    async fn validate_user(
        &self,
        email: &Email,
        raw_password: &str,
    ) -> Result<(), UserStoreError> {
        let user = self.get_user(email).await?;
        user.password
            .verify_raw_password(raw_password)
            .await
            .map_err(|_| UserStoreError::InvalidCredentials)
    }
}
