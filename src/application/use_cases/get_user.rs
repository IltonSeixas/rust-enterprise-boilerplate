use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::dtos::UserResponse,
    domain::{errors::DomainError, repositories::UserRepository},
};

pub struct GetUser {
    user_repo: Arc<dyn UserRepository>,
}

impl GetUser {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    pub async fn execute(&self, id: Uuid) -> Result<UserResponse, DomainError> {
        let user = self
            .user_repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        Ok(UserResponse {
            id: user.id().value(),
            email: user.email().value().to_owned(),
            name: user.name().to_owned(),
            role: user.role().clone(),
            is_active: user.is_active(),
            created_at: user.created_at(),
            updated_at: user.updated_at(),
        })
    }
}
