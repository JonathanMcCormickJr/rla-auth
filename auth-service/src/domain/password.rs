use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
};

use color_eyre::eyre::{eyre, Context, Result};
use secrecy::{ExposeSecret, SecretString};

#[derive(Debug, Clone)]
pub struct HashedPassword(SecretString);

impl PartialEq for HashedPassword {
    fn eq(&self, other: &Self) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

impl Eq for HashedPassword {}

impl HashedPassword {
    #[tracing::instrument(name = "HashedPassword Parse", skip_all)]
    pub async fn parse(s: SecretString) -> Result<Self> {
        if validate_password(&s) {
            let hash = compute_password_hash(&s).await?;
            Ok(Self(hash))
        } else {
            Err(eyre!("Failed to parse string to a HashedPassword type"))
        }
    }

    #[tracing::instrument(name = "HashedPassword Parse password hash", skip_all)]
    pub fn parse_password_hash(hash: SecretString) -> Result<Self> {
        if PasswordHash::new(hash.expose_secret()).is_ok() {
            Ok(Self(hash))
        } else {
            Err(eyre!("Failed to parse string to a HashedPassword type"))
        }
    }

    #[tracing::instrument(name = "HashedPassword Verify raw password", skip_all)]
    pub async fn verify_raw_password(
        &self,
        password_candidate: &SecretString,
    ) -> Result<()> {
        let current_span: tracing::Span = tracing::Span::current();
        let password_hash = self.0.expose_secret().to_owned();
        let candidate = password_candidate.expose_secret().to_owned();
        tokio::task::spawn_blocking(move || {
            current_span.in_scope(|| -> Result<()> {
                let expected_password_hash: PasswordHash = PasswordHash::new(&password_hash)
                    .map_err(|e| eyre!(e.to_string()))?;
                Argon2::default()
                    .verify_password(candidate.as_bytes(), &expected_password_hash)
                    .map_err(|e| eyre!(e.to_string()))
                    .wrap_err("failed to verify password hash")
            })
        })
        .await?
    }
}

fn validate_password(s: &SecretString) -> bool {
    s.expose_secret().len() >= 8
}

impl AsRef<SecretString> for HashedPassword {
    fn as_ref(&self) -> &SecretString {
        &self.0
    }
}

#[tracing::instrument(name = "Computing password hash", skip_all)]
pub async fn compute_password_hash(password: &SecretString) -> Result<SecretString> {
    let current_span: tracing::Span = tracing::Span::current();
    let password = password.expose_secret().to_owned();
    tokio::task::spawn_blocking(move || {
        current_span.in_scope(|| -> Result<SecretString> {
            let salt = SaltString::generate(&mut OsRng);
            let params = Params::new(15000, 2, 1, None)
                .map_err(|e| eyre!(e.to_string()))?;
            let hash = Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
                .hash_password(password.as_bytes(), &salt)
                .map_err(|e| eyre!(e.to_string()))?
                .to_string();
            Ok(SecretString::new(hash.into_boxed_str()))
        })
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::HashedPassword;
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Algorithm, Argon2, Params, PasswordHasher, Version,
    };
    use fake::faker::internet::en::Password as FakePassword;
    use fake::Fake;
    use quickcheck::Gen;
    use secrecy::{ExposeSecret, SecretString};

    fn secret(s: &str) -> SecretString {
        SecretString::new(s.to_owned().into_boxed_str())
    }

    #[tokio::test]
    async fn empty_string_is_rejected() {
        assert!(HashedPassword::parse(secret("")).await.is_err());
    }

    #[tokio::test]
    async fn string_less_than_8_characters_is_rejected() {
        assert!(HashedPassword::parse(secret("1234567")).await.is_err());
    }

    #[tokio::test]
    async fn valid_password_is_hashed() {
        let hp = HashedPassword::parse(secret("ValidPass123")).await.unwrap();
        assert!(hp.as_ref().expose_secret().starts_with("$argon2id$v=19$"));
    }

    #[test]
    fn can_parse_valid_argon2_hash() {
        let raw_password = "TestPassword123";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        );

        let hash_string = argon2
            .hash_password(raw_password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let hash_password = HashedPassword::parse_password_hash(secret(&hash_string)).unwrap();

        assert_eq!(hash_password.as_ref().expose_secret(), hash_string.as_str());
        assert!(hash_password
            .as_ref()
            .expose_secret()
            .starts_with("$argon2id$v=19$"));
    }

    #[tokio::test]
    async fn parse_password_hash_rejects_malformed_string() {
        assert!(HashedPassword::parse_password_hash(secret("not-a-hash")).is_err());
    }

    #[tokio::test]
    async fn can_verify_raw_password() {
        let raw_password = "TestPassword123";
        let hash_password = HashedPassword::parse(secret(raw_password)).await.unwrap();

        assert!(hash_password
            .verify_raw_password(&secret(raw_password))
            .await
            .is_ok());
        assert!(hash_password
            .verify_raw_password(&secret("wrong_password"))
            .await
            .is_err());
    }

    #[derive(Debug)]
    struct ValidPasswordFixture(SecretString);

    impl Clone for ValidPasswordFixture {
        fn clone(&self) -> Self {
            Self(SecretString::new(
                self.0.expose_secret().to_owned().into_boxed_str(),
            ))
        }
    }

    impl quickcheck::Arbitrary for ValidPasswordFixture {
        fn arbitrary(_g: &mut Gen) -> Self {
            let password: String = FakePassword(8..30).fake();
            Self(SecretString::new(password.into_boxed_str()))
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_passwords_are_parsed_successfully(valid_password: ValidPasswordFixture) -> bool {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async { HashedPassword::parse(valid_password.0).await.is_ok() })
    }
}
