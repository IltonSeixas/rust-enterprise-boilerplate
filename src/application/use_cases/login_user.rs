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
        Self { user_repo, hasher, token_svc }
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

        let valid = self.hasher.verify(&req.password, user.password_hash()).await?;
        if !valid {
            return Err(DomainError::InvalidCredentials);
        }

        let tokens = self.token_svc.generate_pair(user.id().value(), user.role()).await?;

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
