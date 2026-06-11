pub mod auth_handler;
pub mod health_handler;
pub mod metrics_handler;
pub mod user_handler;

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::domain::errors::DomainError;

pub fn error_response(err: DomainError) -> axum::response::Response {
    let (status, message) = match &err {
        DomainError::InvalidEmail
        | DomainError::InvalidPasswordLength
        | DomainError::InvalidName => (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()),
        DomainError::EmailAlreadyExists => (StatusCode::CONFLICT, err.to_string()),
        DomainError::UserNotFound => (StatusCode::NOT_FOUND, err.to_string()),
        DomainError::InvalidCredentials => (StatusCode::UNAUTHORIZED, err.to_string()),
        DomainError::AccountInactive => (StatusCode::FORBIDDEN, err.to_string()),
        DomainError::InsufficientPermissions => (StatusCode::FORBIDDEN, err.to_string()),
        DomainError::Repository(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".into(),
        ),
    };

    (status, Json(json!({ "error": message }))).into_response()
}
