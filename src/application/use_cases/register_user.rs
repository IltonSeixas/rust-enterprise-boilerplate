use std::sync::Arc;

use crate::{
    application::{
        dtos::{AuthResponse, RegisterRequest, UserSummary},
        ports::{PasswordHasher, TokenService},
    },
    domain::{
        entities::{Role, User},
        errors::DomainError,
        repositories::UserRepository,
        value_objects::Email,
    },
};

pub struct RegisterUser {
    user_repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    token_svc: Arc<dyn TokenService>,
}

impl RegisterUser {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        token_svc: Arc<dyn TokenService>,
    ) -> Self {
        Self {
            user_repo,
            hasher,
            token_svc,
        }
    }

    pub async fn execute(&self, req: RegisterRequest) -> Result<AuthResponse, DomainError> {
        if req.password.len() < 12 || req.password.len() > 128 {
            return Err(DomainError::InvalidPasswordLength);
        }

        let email = Email::new(&req.email)?;

        if self.user_repo.find_by_email(&email).await?.is_some() {
            return Err(DomainError::EmailAlreadyExists);
        }

        let hash = self.hasher.hash(&req.password).await?;

        // Attempt to claim the Owner role atomically. If another concurrent
        // registration beat us to it, fall back to Role::User.
        let user_as_owner = User::new(email.clone(), hash.clone(), req.name.clone(), Role::Owner)?;
        let (user, claimed_owner) = if self.user_repo.save_first_owner(&user_as_owner).await? {
            (user_as_owner, true)
        } else {
            let u = User::new(email, hash, req.name, Role::User)?;
            self.user_repo.save(&u).await?;
            (u, false)
        };
        let _ = claimed_owner;

        let tokens = self
            .token_svc
            .generate_pair(user.id().value(), user.role())
            .await?;

        Ok(AuthResponse {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            user: UserSummary {
                id: user.id().value(),
                email: user.email().value().to_owned(),
                name: user.name().to_owned(),
                role: user.role().clone(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use mockall::predicate::*;

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

    mock! {
        pub Hasher {}
        #[async_trait::async_trait]
        impl PasswordHasher for Hasher {
            async fn hash(&self, password: &str) -> Result<crate::domain::value_objects::PasswordHash, DomainError>;
            async fn verify(&self, password: &str, hash: &crate::domain::value_objects::PasswordHash) -> Result<bool, DomainError>;
        }
    }

    mock! {
        pub TokenSvc {}
        #[async_trait::async_trait]
        impl TokenService for TokenSvc {
            async fn generate_pair(&self, user_id: uuid::Uuid, role: &Role) -> Result<crate::application::ports::TokenPair, DomainError>;
            async fn validate_access_token(&self, token: &str) -> Result<crate::application::ports::AccessTokenClaims, DomainError>;
            async fn rotate_refresh_token(&self, old_token: &str, user_id: uuid::Uuid, role: &Role) -> Result<crate::application::ports::TokenPair, DomainError>;
            async fn revoke_refresh_token(&self, token: &str) -> Result<(), DomainError>;
        }
    }

    #[tokio::test]
    async fn rejects_short_password() {
        let repo = MockUserRepo::new();
        let hasher = MockHasher::new();
        let token_svc = MockTokenSvc::new();
        let uc = RegisterUser::new(Arc::new(repo), Arc::new(hasher), Arc::new(token_svc));

        let req = RegisterRequest {
            email: "a@b.com".into(),
            password: "short".into(),
            name: "Test".into(),
        };
        assert_eq!(
            uc.execute(req).await.unwrap_err(),
            DomainError::InvalidPasswordLength
        );
    }

    #[tokio::test]
    async fn rejects_duplicate_email() {
        let mut repo = MockUserRepo::new();
        let hasher = MockHasher::new();
        let token_svc = MockTokenSvc::new();

        repo.expect_find_by_email().returning(|_| {
            let email = Email::new("a@b.com").unwrap();
            let hash = crate::domain::value_objects::PasswordHash::from_phc_string("x".into());
            let user = User::new(email, hash, "x".into(), Role::User).unwrap();
            Ok(Some(user))
        });

        let uc = RegisterUser::new(Arc::new(repo), Arc::new(hasher), Arc::new(token_svc));
        let req = RegisterRequest {
            email: "a@b.com".into(),
            password: "strongpassword1234".into(),
            name: "Test".into(),
        };
        assert_eq!(
            uc.execute(req).await.unwrap_err(),
            DomainError::EmailAlreadyExists
        );
    }

    #[tokio::test]
    async fn first_registration_claims_owner_role() {
        let mut repo = MockUserRepo::new();
        let mut hasher = MockHasher::new();
        let mut token_svc = MockTokenSvc::new();

        repo.expect_find_by_email().returning(|_| Ok(None));
        repo.expect_save_first_owner().returning(|_| Ok(true));

        hasher.expect_hash().returning(|_| {
            Ok(crate::domain::value_objects::PasswordHash::from_phc_string(
                "$argon2id$v=19$m=65536,t=2,p=1$salt$hash".into(),
            ))
        });

        token_svc.expect_generate_pair().returning(|_, _| {
            Ok(crate::application::ports::TokenPair {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
            })
        });

        let uc = RegisterUser::new(Arc::new(repo), Arc::new(hasher), Arc::new(token_svc));
        let req = RegisterRequest {
            email: "owner@example.com".into(),
            password: "strongpassword1234".into(),
            name: "Owner".into(),
        };
        let result = uc.execute(req).await.unwrap();
        assert_eq!(result.user.role, Role::Owner);
    }

    #[tokio::test]
    async fn second_owner_registration_falls_back_to_member() {
        let mut repo = MockUserRepo::new();
        let mut hasher = MockHasher::new();
        let mut token_svc = MockTokenSvc::new();

        repo.expect_find_by_email().returning(|_| Ok(None));
        repo.expect_save_first_owner().returning(|_| Ok(false));
        repo.expect_save().returning(|_| Ok(()));

        hasher.expect_hash().returning(|_| {
            Ok(crate::domain::value_objects::PasswordHash::from_phc_string(
                "$argon2id$v=19$m=65536,t=2,p=1$salt$hash".into(),
            ))
        });

        token_svc.expect_generate_pair().returning(|_, _| {
            Ok(crate::application::ports::TokenPair {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
            })
        });

        let uc = RegisterUser::new(Arc::new(repo), Arc::new(hasher), Arc::new(token_svc));
        let req = RegisterRequest {
            email: "member@example.com".into(),
            password: "strongpassword1234".into(),
            name: "Member".into(),
        };
        let result = uc.execute(req).await.unwrap();
        assert_eq!(result.user.role, Role::User);
    }
}
