use crate::domain::errors::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

impl Email {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let trimmed = value.trim().to_lowercase();
        if !Self::is_valid(&trimmed) {
            return Err(DomainError::InvalidEmail);
        }
        Ok(Self(trimmed))
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    fn is_valid(value: &str) -> bool {
        if value.is_empty() || value.len() > 254 {
            return false;
        }
        let parts: Vec<&str> = value.splitn(2, '@').collect();
        if parts.len() != 2 {
            return false;
        }
        let local = parts[0];
        let domain = parts[1];
        !local.is_empty()
            && !domain.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_email_is_accepted() {
        assert!(Email::new("user@example.com").is_ok());
    }

    #[test]
    fn email_is_lowercased_on_construction() {
        let email = Email::new("User@Example.COM").unwrap();
        assert_eq!(email.value(), "user@example.com");
    }

    #[test]
    fn email_without_at_sign_is_rejected() {
        assert_eq!(Email::new("notanemail"), Err(DomainError::InvalidEmail));
    }

    #[test]
    fn email_without_domain_is_rejected() {
        assert_eq!(Email::new("user@"), Err(DomainError::InvalidEmail));
    }

    #[test]
    fn email_without_tld_is_rejected() {
        assert_eq!(Email::new("user@domain"), Err(DomainError::InvalidEmail));
    }

    #[test]
    fn empty_email_is_rejected() {
        assert_eq!(Email::new(""), Err(DomainError::InvalidEmail));
    }

    #[test]
    fn email_exceeding_max_length_is_rejected() {
        let long = format!("{}@example.com", "a".repeat(250));
        assert_eq!(Email::new(&long), Err(DomainError::InvalidEmail));
    }
}
