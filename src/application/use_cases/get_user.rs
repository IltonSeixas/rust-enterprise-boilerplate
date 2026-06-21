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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::{Role, User},
        value_objects::{Email, PasswordHash},
    };
    use mockall::mock;

    mock! {
        pub UserRepo {}
        #[async_trait::async_trait]
        impl UserRepository for UserRepo {
            async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<User>, DomainError>;
            async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError>;
            async fn save(&self, user: &User) -> Result<(), DomainError>;
            async fn delete(&self, id: uuid::Uuid) -> Result<(), DomainError>;
            async fn count(&self) -> Result<u64, DomainError>;
            async fn save_first_owner(&self, user: &User) -> Result<bool, DomainError>;
            async fn find_paginated(&self, offset: u64, limit: u64) -> Result<(Vec<User>, u64), DomainError>;
        }
    }

    fn make_user() -> User {
        let email = Email::new("user@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        User::new(email, hash, "Test User".into(), Role::User).unwrap()
    }

    #[tokio::test]
    async fn rejects_when_user_not_found() {
        let mut repo = MockUserRepo::new();
        repo.expect_find_by_id().returning(|_| Ok(None));

        let uc = GetUser::new(Arc::new(repo));
        assert_eq!(
            uc.execute(Uuid::new_v4()).await.unwrap_err(),
            DomainError::UserNotFound
        );
    }

    #[tokio::test]
    async fn returns_user_response_on_success() {
        let mut repo = MockUserRepo::new();
        let user = make_user();
        let expected_id = user.id().value();
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(user.clone())));

        let uc = GetUser::new(Arc::new(repo));
        let result = uc.execute(expected_id).await.unwrap();
        assert_eq!(result.id, expected_id);
        assert_eq!(result.email, "user@example.com");
        assert_eq!(result.name, "Test User");
        assert_eq!(result.role, Role::User);
        assert!(result.is_active);
    }
}
