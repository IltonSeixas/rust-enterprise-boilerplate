use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::dtos::{ChangeRoleRequest, UserResponse},
    domain::{errors::DomainError, repositories::UserRepository},
};

pub struct ChangeUserRole {
    user_repo: Arc<dyn UserRepository>,
}

impl ChangeUserRole {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    pub async fn execute(
        &self,
        actor_id: Uuid,
        target_id: Uuid,
        req: ChangeRoleRequest,
    ) -> Result<UserResponse, DomainError> {
        let actor = self
            .user_repo
            .find_by_id(actor_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        let mut target = self
            .user_repo
            .find_by_id(target_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        target.change_role(req.role, &actor)?;
        self.user_repo.save(&target).await?;

        Ok(UserResponse {
            id: target.id().value(),
            email: target.email().value().to_owned(),
            name: target.name().to_owned(),
            role: target.role().clone(),
            is_active: target.is_active(),
            created_at: target.created_at(),
            updated_at: target.updated_at(),
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

    fn make_user(role: Role) -> User {
        let email = Email::new("user@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        User::new(email, hash, "Test User".into(), role).unwrap()
    }

    #[tokio::test]
    async fn rejects_when_actor_not_found() {
        let mut repo = MockUserRepo::new();
        repo.expect_find_by_id().returning(|_| Ok(None));

        let uc = ChangeUserRole::new(Arc::new(repo));
        let req = ChangeRoleRequest { role: Role::Admin };
        assert_eq!(
            uc.execute(Uuid::new_v4(), Uuid::new_v4(), req)
                .await
                .unwrap_err(),
            DomainError::UserNotFound
        );
    }

    #[tokio::test]
    async fn rejects_when_target_not_found() {
        let mut repo = MockUserRepo::new();
        let actor = make_user(Role::Owner);

        repo.expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(actor.clone())));
        repo.expect_find_by_id().times(1).returning(|_| Ok(None));

        let uc = ChangeUserRole::new(Arc::new(repo));
        let req = ChangeRoleRequest { role: Role::Admin };
        assert_eq!(
            uc.execute(Uuid::new_v4(), Uuid::new_v4(), req)
                .await
                .unwrap_err(),
            DomainError::UserNotFound
        );
    }

    #[tokio::test]
    async fn rejects_when_actor_lacks_permission() {
        let mut repo = MockUserRepo::new();
        let actor = make_user(Role::User);
        let target = make_user(Role::User);

        repo.expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(actor.clone())));
        repo.expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(target.clone())));

        let uc = ChangeUserRole::new(Arc::new(repo));
        let req = ChangeRoleRequest { role: Role::Admin };
        assert_eq!(
            uc.execute(Uuid::new_v4(), Uuid::new_v4(), req)
                .await
                .unwrap_err(),
            DomainError::InsufficientPermissions
        );
    }

    #[tokio::test]
    async fn admin_promotes_user_to_admin_and_persists() {
        let mut repo = MockUserRepo::new();
        let actor = make_user(Role::Owner);
        let target = make_user(Role::User);
        let target_id = target.id().value();

        repo.expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(actor.clone())));
        repo.expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(target.clone())));
        repo.expect_save()
            .withf(|u| u.role() == &Role::Admin)
            .returning(|_| Ok(()));

        let uc = ChangeUserRole::new(Arc::new(repo));
        let req = ChangeRoleRequest { role: Role::Admin };
        let result = uc.execute(Uuid::new_v4(), target_id, req).await.unwrap();
        assert_eq!(result.role, Role::Admin);
        assert_eq!(result.id, target_id);
    }
}
