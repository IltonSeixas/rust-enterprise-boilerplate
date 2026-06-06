use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{dtos::ChangePasswordRequest, ports::PasswordHasher},
    domain::{errors::DomainError, repositories::UserRepository},
};

pub struct ChangePassword {
    user_repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl ChangePassword {
    pub fn new(user_repo: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { user_repo, hasher }
    }

    pub async fn execute(&self, id: Uuid, req: ChangePasswordRequest) -> Result<(), DomainError> {
        if req.new_password.len() < 12 || req.new_password.len() > 128 {
            return Err(DomainError::InvalidPasswordLength);
        }

        let mut user = self
            .user_repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        let valid = self.hasher.verify(&req.current_password, user.password_hash()).await?;
        if !valid {
            return Err(DomainError::InvalidCredentials);
        }

        let new_hash = self.hasher.hash(&req.new_password).await?;
        user.update_password(new_hash);
        self.user_repo.save(&user).await?;

        Ok(())
    }
}
