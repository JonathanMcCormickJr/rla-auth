use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashedPassword(String);

impl HashedPassword {
    pub async fn parse(password: String) -> Result<Self, String> {
        if password.chars().count() < 8 {
            return Err("Password must be at least 8 characters long".to_string());
        }
        let hash = compute_password_hash(password).await?;
        Ok(HashedPassword(hash))
    }

    pub fn parse_password_hash(hash: String) -> Result<HashedPassword, String> {
        PasswordHash::new(&hash).map_err(|e| e.to_string())?;
        Ok(HashedPassword(hash))
    }

    pub async fn verify_raw_password(&self, password_candidate: &str) -> Result<(), String> {
        let password_hash = self.0.clone();
        let candidate = password_candidate.to_owned();
        tokio::task::spawn_blocking(move || {
            let expected = PasswordHash::new(&password_hash).map_err(|e| e.to_string())?;
            Argon2::default()
                .verify_password(candidate.as_bytes(), &expected)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}

impl AsRef<str> for HashedPassword {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

async fn compute_password_hash(password: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let params = Params::new(15000, 2, 1, None).map_err(|e| e.to_string())?;
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::HashedPassword;
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Algorithm, Argon2, Params, PasswordHasher, Version,
    };

    #[tokio::test]
    async fn empty_string_is_rejected() {
        assert!(HashedPassword::parse("".to_owned()).await.is_err());
    }

    #[tokio::test]
    async fn string_less_than_8_characters_is_rejected() {
        assert!(HashedPassword::parse("1234567".to_owned()).await.is_err());
    }

    #[tokio::test]
    async fn valid_password_is_hashed() {
        let hp = HashedPassword::parse("ValidPass123".to_owned())
            .await
            .unwrap();
        assert!(hp.as_ref().starts_with("$argon2id$v=19$"));
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

        let hash_password = HashedPassword::parse_password_hash(hash_string.clone()).unwrap();

        assert_eq!(hash_password.as_ref(), hash_string.as_str());
        assert!(hash_password.as_ref().starts_with("$argon2id$v=19$"));
    }

    #[tokio::test]
    async fn parse_password_hash_rejects_malformed_string() {
        assert!(HashedPassword::parse_password_hash("not-a-hash".to_owned()).is_err());
    }

    #[tokio::test]
    async fn can_verify_raw_password() {
        let raw_password = "TestPassword123";
        let hash_password = HashedPassword::parse(raw_password.to_owned()).await.unwrap();

        assert!(hash_password.verify_raw_password(raw_password).await.is_ok());
        assert!(hash_password
            .verify_raw_password("wrong_password")
            .await
            .is_err());
    }
}
