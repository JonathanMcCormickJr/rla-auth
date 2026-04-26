use std::hash::Hash;

use color_eyre::eyre::{eyre, Result};
use secrecy::{ExposeSecret, SecretString};
use validator::ValidateEmail;

#[derive(Clone, Debug)]
pub struct Email(SecretString);

impl PartialEq for Email {
    fn eq(&self, other: &Self) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

impl Eq for Email {}

impl Hash for Email {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.expose_secret().hash(state);
    }
}

impl Email {
    pub fn parse(s: SecretString) -> Result<Email> {
        if s.expose_secret().validate_email() {
            Ok(Self(s))
        } else {
            Err(eyre!("{} is not a valid email.", s.expose_secret()))
        }
    }
}

impl AsRef<SecretString> for Email {
    fn as_ref(&self) -> &SecretString {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Email;

    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use quickcheck::Gen;
    use secrecy::{ExposeSecret, SecretString};

    fn secret(s: &str) -> SecretString {
        SecretString::new(s.to_owned().into_boxed_str())
    }

    #[test]
    fn empty_string_is_rejected() {
        assert!(Email::parse(secret("")).is_err());
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        assert!(Email::parse(secret("ursuladomain.com")).is_err());
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        assert!(Email::parse(secret("@domain.com")).is_err());
    }

    #[test]
    fn use_email_as_ref() {
        let email_str = "test@example.com";
        let email = Email::parse(secret(email_str)).unwrap();
        assert_eq!(email.as_ref().expose_secret(), email_str);
    }

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(_g: &mut Gen) -> Self {
            let email: String = SafeEmail().fake();
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        Email::parse(SecretString::new(valid_email.0.into_boxed_str())).is_ok()
    }
}
