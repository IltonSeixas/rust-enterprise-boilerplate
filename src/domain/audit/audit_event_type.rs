use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventType {
    UserRegistered,
    LoginSucceeded,
    LoginFailed,
    PasswordChanged,
    RoleChanged,
    TokenRefreshed,
}

impl fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::UserRegistered => "user_registered",
            Self::LoginSucceeded => "login_succeeded",
            Self::LoginFailed => "login_failed",
            Self::PasswordChanged => "password_changed",
            Self::RoleChanged => "role_changed",
            Self::TokenRefreshed => "token_refreshed",
        };
        f.write_str(s)
    }
}
