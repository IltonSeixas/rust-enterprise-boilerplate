use async_trait::async_trait;

use crate::domain::{errors::DomainError, value_objects::PasswordHash};

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> Result<PasswordHash, DomainError>;
    async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, DomainError>;
}
