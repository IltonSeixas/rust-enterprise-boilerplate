use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash as Argon2Hash, PasswordHasher as _, PasswordVerifier,
};
use async_trait::async_trait;

use crate::{
    application::ports::PasswordHasher,
    domain::{errors::DomainError, value_objects::PasswordHash},
};

pub struct Argon2Hasher {
    argon2: Argon2<'static>,
}

impl Argon2Hasher {
    pub fn new() -> Self {
        use argon2::{Algorithm, Params, Version};
        let params = Params::new(65536, 3, 4, None).expect("valid argon2 params");
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        Self { argon2 }
    }
}

impl Default for Argon2Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PasswordHasher for Argon2Hasher {
    async fn hash(&self, password: &str) -> Result<PasswordHash, DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        let phc = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| DomainError::Repository(e.to_string()))?
            .to_string();
        Ok(PasswordHash::from_phc_string(phc))
    }

    async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, DomainError> {
        let parsed =
            Argon2Hash::new(hash.value()).map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(self.argon2.verify_password(password.as_bytes(), &parsed).is_ok())
    }
}
