use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{
    application::ports::TokenService,
    domain::{entities::Role, repositories::UserRepository},
};

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub role: Role,
}

pub struct AuthMiddlewareState {
    pub token_svc: Arc<dyn TokenService>,
    pub user_repo: Arc<dyn UserRepository>,
}

impl Clone for AuthMiddlewareState {
    fn clone(&self) -> Self {
        Self {
            token_svc: Arc::clone(&self.token_svc),
            user_repo: Arc::clone(&self.user_repo),
        }
    }
}

pub async fn require_auth(
    State(state): State<AuthMiddlewareState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = state
        .token_svc
        .validate_access_token(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Verify the account is still active on every request. This ensures that
    // deactivated accounts are blocked immediately rather than waiting for
    // the access token to expire.
    let user = state
        .user_repo
        .find_by_id(claims.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !user.is_active() {
        return Err(StatusCode::FORBIDDEN);
    }

    req.extensions_mut().insert(AuthenticatedUser {
        id: claims.user_id,
        role: claims.role,
    });

    Ok(next.run(req).await)
}
