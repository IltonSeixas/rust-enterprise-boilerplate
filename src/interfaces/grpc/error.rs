use tonic::{Code, Status};

use crate::domain::errors::DomainError;

pub fn to_status(err: DomainError) -> Status {
    let code = match &err {
        DomainError::InvalidEmail
        | DomainError::InvalidPasswordLength
        | DomainError::InvalidName
        | DomainError::InvalidRole => Code::InvalidArgument,
        DomainError::EmailAlreadyExists => Code::AlreadyExists,
        DomainError::UserNotFound => Code::NotFound,
        DomainError::InvalidCredentials => Code::Unauthenticated,
        DomainError::AccountInactive | DomainError::InsufficientPermissions => {
            Code::PermissionDenied
        }
        DomainError::Repository(_) => Code::Internal,
    };

    Status::new(code, err.to_string())
}
