use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::Role, errors::DomainError};

pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

pub struct AccessTokenClaims {
    pub user_id: Uuid,
    pub role: Role,
}

#[async_trait]
pub trait TokenService: Send + Sync {
    async fn generate_pair(&self, user_id: Uuid, role: &Role) -> Result<TokenPair, DomainError>;
    async fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims, DomainError>;
    async fn find_user_id_by_refresh_token(&self, token: &str)
        -> Result<Option<Uuid>, DomainError>;
    async fn rotate_refresh_token(
        &self,
        old_token: &str,
        user_id: Uuid,
        role: &Role,
    ) -> Result<TokenPair, DomainError>;
    async fn revoke_refresh_token(&self, token: &str) -> Result<(), DomainError>;
}
