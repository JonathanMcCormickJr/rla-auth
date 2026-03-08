use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Email(String);

impl Email {
    pub fn parse(email: &str) -> Result<Self, &'static str> {
        if is_valid_email(email) {
            Ok(Email(email.to_string()))
        } else {
            Err("Invalid email format")
        }
    }
}

fn is_valid_email(email: &str) -> bool {
    if email.is_empty() || !email.is_ascii() {
        return false;
    }

    if email.matches('@').count() != 1 {
        return false;
    }

    let (local, domain) = match email.split_once('@') {
        Some(parts) => parts,
        None => return false,
    };

    is_valid_local(local) && is_valid_domain(domain)
}

fn is_valid_local(local: &str) -> bool {
    if local.is_empty() || local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }

    local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '%' | '+' | '-' | '.'))
}

fn is_valid_domain(domain: &str) -> bool {
    if domain.is_empty() || domain.contains("..") {
        return false;
    }

    let mut labels = domain.split('.');
    let mut saw_dot = false;
    let mut last_label = "";

    for label in labels.by_ref() {
        if !is_valid_domain_label(label) {
            return false;
        }

        if !last_label.is_empty() {
            saw_dot = true;
        }

        last_label = label;
    }

    saw_dot
        && !last_label.is_empty()
        && last_label.len() <= 63
        && last_label.chars().all(|c| c.is_ascii_alphabetic())
}

fn is_valid_domain_label(label: &str) -> bool {
    if label.is_empty() || label.starts_with('-') || label.ends_with('-') {
        return false;
    }

    label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::email::Email;

    #[test]
    fn parse_valid_email() {
        let valid_emails = vec![
            "test@example.com",
            "very.common@example.com",
            "user_name@example.co.uk",
            "disposable.style.email.with+symbol@example.com",
            "user%mailbox@example.io",
            "x@example.net",
            "USER123@EXAMPLE.COM",
            "a.b-c_d+e%f@sub-domain.example.org",
            "username@example.c",
        ];

        for valid_email in valid_emails {
            let email = Email::parse(valid_email);
            assert!(
                email.is_ok(),
                "Expected '{valid_email}' to be valid, but parsing failed"
            );
        }
    }

    #[test]
    fn parse_invalid_email() {
        let invalid_emails = vec![
            "",
            " ",
            "plainaddress",
            "@missingusername.com",
            "username@.com",
            "username@com",
            "username@domain..com",
            "hastwoatsigns@@example.com",
            "hasinvalidchars!@example.com",
            "hasspace in@example.com",
            "hasnewline\n@example.com",
            ".startswithdot@example.com",
            "endswithdot.@example.com",
            "double..dot@example.com",
            "username@-example.com",
            "username@example-.com",
            "username@example.123",
            "username@exa_mple.com",
            "username@example.com ",
            " username@example.com",
            "user@sub..example.com",
            "user@localhost",
            "user@[127.0.0.1]",
            "üser@example.com",
        ];

        for invalid_email in invalid_emails {
            let email = Email::parse(invalid_email);
            assert!(
                email.is_err(),
                "Expected '{invalid_email}' to be invalid, but parsing succeeded"
            );
        }
    }

    #[test]
    fn use_email_as_ref() {
        let email_str = "test@example.com";
        let email = Email::parse(email_str).unwrap();
        assert_eq!(email.as_ref(), email_str);
        assert_eq!(email.0, email_str);
        assert_eq!(&email.0, email_str);
    }
}
