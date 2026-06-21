use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum DomainError {
    #[error("invalid email address")]
    InvalidEmail,

    #[error("password must be between 12 and 128 characters")]
    InvalidPasswordLength,

    #[error("name must be between 1 and 100 characters")]
    InvalidName,

    #[error("invalid role")]
    InvalidRole,

    #[error("user not found")]
    UserNotFound,

    #[error("email already registered")]
    EmailAlreadyExists,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("account is inactive")]
    AccountInactive,

    #[error("insufficient permissions")]
    InsufficientPermissions,

    #[error("repository error: {0}")]
    Repository(String),

    #[error("service temporarily unavailable")]
    ServiceUnavailable,
}
