use std::sync::Arc;

use tonic::{Request, Status};
use uuid::Uuid;

use crate::{
    application::ports::TokenService,
    domain::{entities::Role, repositories::UserRepository},
};

pub struct AuthenticatedCaller {
    pub id: Uuid,
    pub role: Role,
}

/// Validates the `authorization: Bearer <token>` metadata entry and ensures the
/// underlying account is still active, mirroring the REST `require_auth` middleware.
pub async fn authenticate<T>(
    req: &Request<T>,
    token_svc: &Arc<dyn TokenService>,
    user_repo: &Arc<dyn UserRepository>,
) -> Result<AuthenticatedCaller, Status> {
    let token = req
        .metadata()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| Status::unauthenticated("missing bearer token"))?;

    let claims = token_svc
        .validate_access_token(token)
        .await
        .map_err(|_| Status::unauthenticated("invalid or expired token"))?;

    let user = user_repo
        .find_by_id(claims.user_id)
        .await
        .map_err(|_| Status::internal("internal server error"))?
        .ok_or_else(|| Status::unauthenticated("invalid or expired token"))?;

    if !user.is_active() {
        return Err(Status::permission_denied("account is inactive"));
    }

    Ok(AuthenticatedCaller {
        id: claims.user_id,
        role: claims.role,
    })
}
