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
