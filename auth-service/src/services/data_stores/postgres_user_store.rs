use color_eyre::eyre::{Context, Report};
use secrecy::{ExposeSecret, SecretString};
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
    #[tracing::instrument(name = "Adding user to PostgreSQL", skip_all)]
    async fn add_user(&mut self, user: User) -> Result<(), UserStoreError> {
        sqlx::query(
            "INSERT INTO users (email, password_hash, requires_2fa) VALUES ($1, $2, $3)",
        )
        .bind(user.email.as_ref().expose_secret())
        .bind(user.password.as_ref().expose_secret())
        .bind(user.requires_2fa)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                UserStoreError::UserAlreadyExists
            }
            other => UserStoreError::UnexpectedError(
                Report::from(other).wrap_err("Failed to insert user into PostgreSQL"),
            ),
        })?;
        Ok(())
    }


    #[tracing::instrument(name = "Retrieving user from PostgreSQL", skip_all)]
    async fn get_user(&self, email: &Email) -> Result<User, UserStoreError> {
        let row = sqlx::query(
            "SELECT email, password_hash, requires_2fa FROM users WHERE email = $1",
        )
        .bind(email.as_ref().expose_secret())
        .fetch_optional(&self.pool)
        .await
        .wrap_err("Failed to query user from PostgreSQL")
        .map_err(UserStoreError::UnexpectedError)?
        .ok_or(UserStoreError::UserNotFound)?;

        let db_email: String = row.get("email");
        let password_hash: String = row.get("password_hash");
        let requires_2fa: bool = row.get("requires_2fa");

        let email = Email::parse(SecretString::new(db_email.into_boxed_str()))
            .map_err(|_| UserStoreError::MalformedCredentials)?;
        let password = HashedPassword::parse_password_hash(SecretString::new(
            password_hash.into_boxed_str(),
        ))
        .map_err(|e| {
            UserStoreError::UnexpectedError(e.wrap_err("Stored password hash failed to parse"))
        })?;

        Ok(User {
            uuid: uuid::Uuid::new_v4(),
            email,
            password,
            requires_2fa,
        })
    }


    #[tracing::instrument(name = "Validating user credentials in PostgreSQL", skip_all)]
    async fn validate_user(
        &self,
        email: &Email,
        raw_password: &SecretString,
    ) -> Result<(), UserStoreError> {
        let user = self.get_user(email).await?;
        user.password
            .verify_raw_password(raw_password)
            .await
            .map_err(|_| UserStoreError::InvalidCredentials)
    }
}
