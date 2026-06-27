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
        DomainError::ServiceUnavailable => Code::Unavailable,
    };

    // Repository errors carry raw database/cache failure strings that must
    // never reach the client; every other variant's Display output is a
    // fixed, safe message.
    let message = match &err {
        DomainError::Repository(_) => "internal server error".to_string(),
        _ => err.to_string(),
    };

    Status::new(code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_errors_never_leak_the_underlying_message() {
        let status = to_status(DomainError::Repository(
            "connection string: postgres://user:secret@db".into(),
        ));

        assert_eq!(status.code(), Code::Internal);
        assert_eq!(status.message(), "internal server error");
    }

    #[test]
    fn domain_validation_errors_pass_through_their_message() {
        let status = to_status(DomainError::InvalidEmail);

        assert_eq!(status.code(), Code::InvalidArgument);
        assert_eq!(status.message(), "invalid email address");
    }
}
