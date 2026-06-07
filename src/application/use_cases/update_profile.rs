use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::dtos::{UpdateProfileRequest, UserResponse},
    domain::{errors::DomainError, repositories::UserRepository},
};

pub struct UpdateProfile {
    user_repo: Arc<dyn UserRepository>,
}

impl UpdateProfile {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    pub async fn execute(
        &self,
        id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<UserResponse, DomainError> {
        let mut user = self
            .user_repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        user.update_name(req.name)?;
        self.user_repo.save(&user).await?;

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
        }
    }

    fn make_user() -> User {
        let email = Email::new("user@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        User::new(email, hash, "Original Name".into(), Role::User).unwrap()
    }

    #[tokio::test]
    async fn rejects_when_user_not_found() {
        let mut repo = MockUserRepo::new();
        repo.expect_find_by_id().returning(|_| Ok(None));

        let uc = UpdateProfile::new(Arc::new(repo));
        let req = UpdateProfileRequest { name: "New Name".into() };
        assert_eq!(uc.execute(Uuid::new_v4(), req).await.unwrap_err(), DomainError::UserNotFound);
    }

    #[tokio::test]
    async fn rejects_invalid_name() {
        let mut repo = MockUserRepo::new();
        let user = make_user();
        repo.expect_find_by_id().returning(move |_| Ok(Some(user.clone())));

        let uc = UpdateProfile::new(Arc::new(repo));
        let req = UpdateProfileRequest { name: "   ".into() };
        assert_eq!(uc.execute(Uuid::new_v4(), req).await.unwrap_err(), DomainError::InvalidName);
    }

    #[tokio::test]
    async fn updates_name_and_persists() {
        let mut repo = MockUserRepo::new();
        let user = make_user();
        let expected_id = user.id().value();
        repo.expect_find_by_id().returning(move |_| Ok(Some(user.clone())));
        repo.expect_save()
            .withf(|u| u.name() == "New Name")
            .returning(|_| Ok(()));

        let uc = UpdateProfile::new(Arc::new(repo));
        let req = UpdateProfileRequest { name: "New Name".into() };
        let result = uc.execute(expected_id, req).await.unwrap();
        assert_eq!(result.name, "New Name");
    }
}
