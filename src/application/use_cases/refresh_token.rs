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
        Self { user_repo, token_svc }
    }

    pub async fn execute(&self, req: RefreshTokenRequest) -> Result<AuthResponse, DomainError> {
        let claims = self
            .token_svc
            .validate_access_token(&req.refresh_token)
            .await
            .map_err(|_| DomainError::InvalidCredentials)?;

        let user = self
            .user_repo
            .find_by_id(claims.user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        if !user.is_active() {
            self.token_svc.revoke_refresh_token(&req.refresh_token).await?;
            return Err(DomainError::AccountInactive);
        }

        let tokens = self
            .token_svc
            .rotate_refresh_token(&req.refresh_token, claims.user_id, user.role())
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
