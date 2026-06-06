use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::User, errors::DomainError, value_objects::Email};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
    #[allow(dead_code)]
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    #[allow(dead_code)]
    async fn count(&self) -> Result<u64, DomainError>;

    /// Atomically saves `user` only if no Owner exists yet.
    /// Returns `Ok(true)` on success, `Ok(false)` if an Owner already existed.
    async fn save_first_owner(&self, user: &User) -> Result<bool, DomainError>;
}
