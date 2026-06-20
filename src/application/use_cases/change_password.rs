use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        dtos::ChangePasswordRequest,
        ports::{AuditPort, PasswordHasher},
    },
    domain::{
        audit::{AuditEvent, AuditEventType},
        errors::DomainError,
        repositories::UserRepository,
    },
};

pub struct ChangePassword {
    user_repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    audit: Arc<dyn AuditPort>,
}

impl ChangePassword {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        audit: Arc<dyn AuditPort>,
    ) -> Self {
        Self {
            user_repo,
            hasher,
            audit,
        }
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

        let valid = self
            .hasher
            .verify(&req.current_password, user.password_hash())
            .await?;
        if !valid {
            return Err(DomainError::InvalidCredentials);
        }

        let new_hash = self.hasher.hash(&req.new_password).await?;
        user.update_password(new_hash);
        self.user_repo.save(&user).await?;

        self.audit
            .record(AuditEvent::new(
                AuditEventType::PasswordChanged,
                Some(id),
                Some(id),
                String::new(),
            ))
            .await;

        Ok(())
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

    mock! {
        pub Hasher {}
        #[async_trait::async_trait]
        impl PasswordHasher for Hasher {
            async fn hash(&self, password: &str) -> Result<PasswordHash, DomainError>;
            async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, DomainError>;
        }
    }

    mock! {
        pub Audit {}
        #[async_trait::async_trait]
        impl AuditPort for Audit {
            async fn record(&self, event: AuditEvent);
        }
    }

    fn noop_audit() -> MockAudit {
        let mut audit = MockAudit::new();
        audit.expect_record().returning(|_| ());
        audit
    }

    fn make_user() -> User {
        let email = Email::new("user@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$old".into());
        User::new(email, hash, "Test User".into(), Role::User).unwrap()
    }

    fn request(current: &str, new: &str) -> ChangePasswordRequest {
        ChangePasswordRequest {
            current_password: current.into(),
            new_password: new.into(),
        }
    }

    #[tokio::test]
    async fn rejects_short_new_password() {
        let repo = MockUserRepo::new();
        let hasher = MockHasher::new();
        let audit = MockAudit::new();

        let uc = ChangePassword::new(Arc::new(repo), Arc::new(hasher), Arc::new(audit));
        let result = uc
            .execute(Uuid::new_v4(), request("currentpassword1", "short"))
            .await;
        assert_eq!(result.unwrap_err(), DomainError::InvalidPasswordLength);
    }

    #[tokio::test]
    async fn rejects_when_user_not_found() {
        let mut repo = MockUserRepo::new();
        let hasher = MockHasher::new();
        let audit = MockAudit::new();
        repo.expect_find_by_id().returning(|_| Ok(None));

        let uc = ChangePassword::new(Arc::new(repo), Arc::new(hasher), Arc::new(audit));
        let result = uc
            .execute(
                Uuid::new_v4(),
                request("currentpassword1", "newpassword1234"),
            )
            .await;
        assert_eq!(result.unwrap_err(), DomainError::UserNotFound);
    }

    #[tokio::test]
    async fn rejects_wrong_current_password() {
        let mut repo = MockUserRepo::new();
        let mut hasher = MockHasher::new();
        let audit = MockAudit::new();
        let user = make_user();
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(user.clone())));
        hasher.expect_verify().returning(|_, _| Ok(false));

        let uc = ChangePassword::new(Arc::new(repo), Arc::new(hasher), Arc::new(audit));
        let result = uc
            .execute(Uuid::new_v4(), request("wrongpassword1", "newpassword1234"))
            .await;
        assert_eq!(result.unwrap_err(), DomainError::InvalidCredentials);
    }

    #[tokio::test]
    async fn updates_password_hash_on_success() {
        let mut repo = MockUserRepo::new();
        let mut hasher = MockHasher::new();
        let audit = noop_audit();
        let user = make_user();
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(user.clone())));
        hasher.expect_verify().returning(|_, _| Ok(true));
        hasher
            .expect_hash()
            .returning(|_| Ok(PasswordHash::from_phc_string("$argon2id$v=19$new".into())));
        repo.expect_save()
            .withf(|u| u.password_hash().value() == "$argon2id$v=19$new")
            .returning(|_| Ok(()));

        let uc = ChangePassword::new(Arc::new(repo), Arc::new(hasher), Arc::new(audit));
        let result = uc
            .execute(
                Uuid::new_v4(),
                request("currentpassword1", "newpassword1234"),
            )
            .await;
        assert!(result.is_ok());
    }
}
