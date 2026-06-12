use std::sync::Arc;

use crate::{
    application::{
        dtos::{AuthResponse, LoginRequest, UserSummary},
        ports::{PasswordHasher, TokenService},
    },
    domain::{errors::DomainError, repositories::UserRepository, value_objects::Email},
};

pub struct LoginUser {
    user_repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    token_svc: Arc<dyn TokenService>,
}

impl LoginUser {
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

    pub async fn execute(&self, req: LoginRequest) -> Result<AuthResponse, DomainError> {
        let email = Email::new(&req.email)?;

        let user = self
            .user_repo
            .find_by_email(&email)
            .await?
            .ok_or(DomainError::InvalidCredentials)?;

        if !user.is_active() {
            return Err(DomainError::AccountInactive);
        }

        let valid = self
            .hasher
            .verify(&req.password, user.password_hash())
            .await?;
        if !valid {
            return Err(DomainError::InvalidCredentials);
        }

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
    use crate::domain::{
        entities::{Role, User},
        value_objects::PasswordHash,
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

    mock! {
        pub Hasher {}
        #[async_trait::async_trait]
        impl PasswordHasher for Hasher {
            async fn hash(&self, password: &str) -> Result<PasswordHash, DomainError>;
            async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, DomainError>;
        }
    }

    mock! {
        pub TokenSvc {}
        #[async_trait::async_trait]
        impl TokenService for TokenSvc {
            async fn generate_pair(&self, user_id: uuid::Uuid, role: &Role) -> Result<crate::application::ports::TokenPair, DomainError>;
            async fn validate_access_token(&self, token: &str) -> Result<crate::application::ports::AccessTokenClaims, DomainError>;
            async fn find_user_id_by_refresh_token(&self, token: &str) -> Result<Option<uuid::Uuid>, DomainError>;
            async fn rotate_refresh_token(&self, old_token: &str, user_id: uuid::Uuid, role: &Role) -> Result<crate::application::ports::TokenPair, DomainError>;
            async fn revoke_refresh_token(&self, token: &str) -> Result<(), DomainError>;
        }
    }

    fn make_user(role: Role, active: bool) -> User {
        let email = Email::new("user@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        let mut user = User::new(email, hash, "Test User".into(), role).unwrap();
        if !active {
            user.deactivate();
        }
        user
    }

    fn login_request() -> LoginRequest {
        LoginRequest {
            email: "user@example.com".into(),
            password: "strongpassword1234".into(),
        }
    }

    #[tokio::test]
    async fn rejects_unknown_email() {
        let mut repo = MockUserRepo::new();
        let hasher = MockHasher::new();
        let token_svc = MockTokenSvc::new();

        repo.expect_find_by_email().returning(|_| Ok(None));

        let uc = LoginUser::new(Arc::new(repo), Arc::new(hasher), Arc::new(token_svc));
        assert_eq!(
            uc.execute(login_request()).await.unwrap_err(),
            DomainError::InvalidCredentials
        );
    }

    #[tokio::test]
    async fn rejects_inactive_account() {
        let mut repo = MockUserRepo::new();
        let hasher = MockHasher::new();
        let token_svc = MockTokenSvc::new();

        repo.expect_find_by_email()
            .returning(|_| Ok(Some(make_user(Role::User, false))));

        let uc = LoginUser::new(Arc::new(repo), Arc::new(hasher), Arc::new(token_svc));
        assert_eq!(
            uc.execute(login_request()).await.unwrap_err(),
            DomainError::AccountInactive
        );
    }

    #[tokio::test]
    async fn rejects_wrong_password() {
        let mut repo = MockUserRepo::new();
        let mut hasher = MockHasher::new();
        let token_svc = MockTokenSvc::new();

        repo.expect_find_by_email()
            .returning(|_| Ok(Some(make_user(Role::User, true))));
        hasher.expect_verify().returning(|_, _| Ok(false));

        let uc = LoginUser::new(Arc::new(repo), Arc::new(hasher), Arc::new(token_svc));
        assert_eq!(
            uc.execute(login_request()).await.unwrap_err(),
            DomainError::InvalidCredentials
        );
    }

    #[tokio::test]
    async fn returns_tokens_on_success() {
        let mut repo = MockUserRepo::new();
        let mut hasher = MockHasher::new();
        let mut token_svc = MockTokenSvc::new();

        repo.expect_find_by_email()
            .returning(|_| Ok(Some(make_user(Role::User, true))));
        hasher.expect_verify().returning(|_, _| Ok(true));
        token_svc.expect_generate_pair().returning(|_, _| {
            Ok(crate::application::ports::TokenPair {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
            })
        });

        let uc = LoginUser::new(Arc::new(repo), Arc::new(hasher), Arc::new(token_svc));
        let result = uc.execute(login_request()).await.unwrap();
        assert_eq!(result.access_token, "access");
        assert_eq!(result.refresh_token, "refresh");
        assert_eq!(result.user.email, "user@example.com");
    }
}
