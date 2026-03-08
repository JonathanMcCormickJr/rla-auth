// Passwords must be at least 8 characters long,
// contain at least one uppercase letter, one lowercase letter, one digit, and one special character.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Password(String);

impl Password {
    pub fn parse(password: &str) -> Result<Self, &'static str> {
        let has_min_length = password.chars().count() >= 8;
        let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_ascii_alphanumeric());

        if has_min_length && has_uppercase && has_lowercase && has_digit && has_special {
            Ok(Password(password.to_string()))
        } else {
            Err("Password must be at least 8 characters long, contain at least one uppercase letter, one lowercase letter, one digit, and one special character")
        }
    }
}

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::password::Password;

    #[test]
    fn parse_valid_password() {
        let valid_passwords = vec![
            ")Password123",
            "P@ssw0rd!",
            "123cC45W@678",
            "Abcdef#gh1",
            "A1b2C3d4E5f6G7h8I9j0K!@#$%^&*()_+-=[]{}|;:,.<>?/~`",
            "%f3gegnS8",
            "98jsvhju9adufhvdf89vjsj9v98H*Fh8vh8sv8(HSSV(J)DVISD)jsdivjP)S(VHhuSD)HV*g}Sv{VS)DijsDV]sjdv]sjvijdvs[vi{SDvh[s(DUHVs9huvdn{SHDiosNdv[)DNV{)DSVNH0Sdvhn[0VND{SNDV{S)duvs(PVHUWY*GYEF*YOGuf~!~(````92uehhr\"fviefee0"
        ];
        for valid_password in valid_passwords {
            let password = Password::parse(valid_password);
            assert!(
                password.is_ok(),
                "Expected '{valid_password}' to be valid, but parsing failed"
            );
        }
    }

    #[test]
    fn parse_invalid_password() {
        let invalid_passwords = vec![
            "short",
            "nouppercase1!",
            "NOLOWERCASE1!",
            "NoSpecialChar1",
            "NoDigit!@#",
            "     ",
            "password",
            "PASSWORD123",
            "Passw0rd", // missing special character
        ];
        for invalid_password in invalid_passwords {
            let password = Password::parse(invalid_password);
            assert!(
                password.is_err(),
                "Expected '{invalid_password}' to be invalid, but parsing succeeded"
            );
        }
    }

    #[test]
    fn use_password_as_ref() {
        let password_str = "P@ssw0rd!";
        let password = Password::parse(password_str).unwrap();
        assert_eq!(password.as_ref(), password_str);
        assert_eq!(password.0, password_str);
        assert_eq!(&password.0, password_str);
    }
}
