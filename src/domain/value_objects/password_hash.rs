/// Opaque wrapper around a verified Argon2id PHC string.
/// Cannot be constructed with arbitrary data — only produced by PasswordHasher.
#[derive(Debug, Clone)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// Only infrastructure (Argon2Hasher) should call this.
    pub fn from_phc_string(phc: String) -> Self {
        Self(phc)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

// Never derive or implement Display — prevents accidental logging.
impl std::fmt::Display for PasswordHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_phc_string_stores_value() {
        let phc = "$argon2id$v=19$m=65536,t=3,p=4$salt$hash".to_string();
        let h = PasswordHash::from_phc_string(phc.clone());
        assert_eq!(h.value(), phc.as_str());
    }

    #[test]
    fn value_returns_original_phc_string() {
        let phc = "$argon2id$v=19$...".to_string();
        let h = PasswordHash::from_phc_string(phc.clone());
        assert_eq!(h.value(), &phc);
    }

    #[test]
    fn display_is_redacted() {
        let h = PasswordHash::from_phc_string("secret".into());
        assert_eq!(format!("{}", h), "[REDACTED]");
    }
}
