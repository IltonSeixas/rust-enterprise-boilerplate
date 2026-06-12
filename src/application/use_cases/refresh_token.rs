use std::sync::Arc;

use crate::{
    application::{
        dtos::{AuthResponse, RefreshTokenRequest, UserSummary},
        ports::TokenService,
    },
    domain::{errors::DomainError, repositories::UserRepository},
};

pub struct RefreshTokenUseCase {
    user_repo: Arc<dyn UserRepository>,
    token_svc: Arc<dyn TokenService>,
}

impl RefreshTokenUseCase {
    pub fn new(user_repo: Arc<dyn UserRepository>, token_svc: Arc<dyn TokenService>) -> Self {
        Self {
            user_repo,
            token_svc,
        }
    }

    pub async fn execute(&self, req: RefreshTokenRequest) -> Result<AuthResponse, DomainError> {
        let user_id = self
            .token_svc
            .find_user_id_by_refresh_token(&req.refresh_token)
            .await?
            .ok_or(DomainError::InvalidCredentials)?;

        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        if !user.is_active() {
            self.token_svc
                .revoke_refresh_token(&req.refresh_token)
                .await?;
            return Err(DomainError::AccountInactive);
        }

        let tokens = self
            .token_svc
            .rotate_refresh_token(&req.refresh_token, user_id, user.role())
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
    use crate::{
        application::ports::TokenPair,
        domain::{
            entities::Role,
            value_objects::{Email, PasswordHash, UserId},
        },
    };
    use mockall::mock;
    use uuid::Uuid;

    mock! {
        pub UserRepo {}
        #[async_trait::async_trait]
        impl UserRepository for UserRepo {
            async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<crate::domain::entities::User>, DomainError>;
            async fn find_by_email(&self, email: &Email) -> Result<Option<crate::domain::entities::User>, DomainError>;
            async fn save(&self, user: &crate::domain::entities::User) -> Result<(), DomainError>;
            async fn delete(&self, id: uuid::Uuid) -> Result<(), DomainError>;
            async fn count(&self) -> Result<u64, DomainError>;
            async fn save_first_owner(&self, user: &crate::domain::entities::User) -> Result<bool, DomainError>;
        }
    }

    mock! {
        pub TokenSvc {}
        #[async_trait::async_trait]
        impl TokenService for TokenSvc {
            async fn generate_pair(&self, user_id: uuid::Uuid, role: &Role) -> Result<TokenPair, DomainError>;
            async fn validate_access_token(&self, token: &str) -> Result<crate::application::ports::AccessTokenClaims, DomainError>;
            async fn find_user_id_by_refresh_token(&self, token: &str) -> Result<Option<uuid::Uuid>, DomainError>;
            async fn rotate_refresh_token(&self, old_token: &str, user_id: uuid::Uuid, role: &Role) -> Result<TokenPair, DomainError>;
            async fn revoke_refresh_token(&self, token: &str) -> Result<(), DomainError>;
        }
    }

    fn make_user(id: Uuid, active: bool) -> crate::domain::entities::User {
        let email = Email::new("user@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        let now = chrono::Utc::now();
        crate::domain::entities::User::reconstitute(crate::domain::entities::user::UserSnapshot {
            id: UserId::from_uuid(id),
            email,
            password_hash: hash,
            name: "Test User".into(),
            role: Role::User,
            is_active: active,
            created_at: now,
            updated_at: now,
        })
    }

    fn request() -> RefreshTokenRequest {
        RefreshTokenRequest {
            refresh_token: "old-refresh-token".into(),
        }
    }

    #[tokio::test]
    async fn rejects_unknown_refresh_token() {
        let repo = MockUserRepo::new();
        let mut token_svc = MockTokenSvc::new();

        token_svc
            .expect_find_user_id_by_refresh_token()
            .returning(|_| Ok(None));

        let uc = RefreshTokenUseCase::new(Arc::new(repo), Arc::new(token_svc));
        assert_eq!(
            uc.execute(request()).await.unwrap_err(),
            DomainError::InvalidCredentials
        );
    }

    #[tokio::test]
    async fn rejects_when_user_no_longer_exists() {
        let mut repo = MockUserRepo::new();
        let mut token_svc = MockTokenSvc::new();
        let user_id = Uuid::new_v4();

        token_svc
            .expect_find_user_id_by_refresh_token()
            .returning(move |_| Ok(Some(user_id)));
        repo.expect_find_by_id().returning(|_| Ok(None));

        let uc = RefreshTokenUseCase::new(Arc::new(repo), Arc::new(token_svc));
        assert_eq!(
            uc.execute(request()).await.unwrap_err(),
            DomainError::UserNotFound
        );
    }

    #[tokio::test]
    async fn revokes_token_and_rejects_inactive_account() {
        let mut repo = MockUserRepo::new();
        let mut token_svc = MockTokenSvc::new();
        let user_id = Uuid::new_v4();

        token_svc
            .expect_find_user_id_by_refresh_token()
            .returning(move |_| Ok(Some(user_id)));
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(make_user(user_id, false))));
        token_svc
            .expect_revoke_refresh_token()
            .returning(|_| Ok(()));

        let uc = RefreshTokenUseCase::new(Arc::new(repo), Arc::new(token_svc));
        assert_eq!(
            uc.execute(request()).await.unwrap_err(),
            DomainError::AccountInactive
        );
    }

    #[tokio::test]
    async fn rotates_token_pair_on_success() {
        let mut repo = MockUserRepo::new();
        let mut token_svc = MockTokenSvc::new();
        let user_id = Uuid::new_v4();

        token_svc
            .expect_find_user_id_by_refresh_token()
            .returning(move |_| Ok(Some(user_id)));
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(make_user(user_id, true))));
        token_svc
            .expect_rotate_refresh_token()
            .returning(|_, _, _| {
                Ok(TokenPair {
                    access_token: "new-access".into(),
                    refresh_token: "new-refresh".into(),
                })
            });

        let uc = RefreshTokenUseCase::new(Arc::new(repo), Arc::new(token_svc));
        let result = uc.execute(request()).await.unwrap();
        assert_eq!(result.access_token, "new-access");
        assert_eq!(result.refresh_token, "new-refresh");
    }
}
