use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    errors::DomainError,
    value_objects::{Email, PasswordHash, UserId},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
    User,
}

impl Role {
    pub fn can_manage_roles(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_promote_to(&self, target: &Role) -> bool {
        matches!((self, target), (Role::Owner, _) | (Role::Admin, Role::User))
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Owner => write!(f, "owner"),
            Role::Admin => write!(f, "admin"),
            Role::User => write!(f, "user"),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(Role::Owner),
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            _ => Err(DomainError::InvalidRole),
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    email: Email,
    password_hash: PasswordHash,
    name: String,
    role: Role,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Field set used to rehydrate a `User` from persistence via `User::reconstitute`.
#[allow(dead_code)]
pub struct UserSnapshot {
    pub id: UserId,
    pub email: Email,
    pub password_hash: PasswordHash,
    pub name: String,
    pub role: Role,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(
        email: Email,
        password_hash: PasswordHash,
        name: String,
        role: Role,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;
        let now = Utc::now();
        Ok(Self {
            id: UserId::new(),
            email,
            password_hash,
            name,
            role,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    #[allow(dead_code)]
    pub fn reconstitute(snapshot: UserSnapshot) -> Self {
        Self {
            id: snapshot.id,
            email: snapshot.email,
            password_hash: snapshot.password_hash,
            name: snapshot.name,
            role: snapshot.role,
            is_active: snapshot.is_active,
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
        }
    }

    pub fn update_name(&mut self, name: String) -> Result<(), DomainError> {
        Self::validate_name(&name)?;
        self.name = name;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_password(&mut self, hash: PasswordHash) {
        self.password_hash = hash;
        self.updated_at = Utc::now();
    }

    pub fn change_role(&mut self, new_role: Role, actor: &User) -> Result<(), DomainError> {
        if !actor.role.can_promote_to(&new_role) {
            return Err(DomainError::InsufficientPermissions);
        }
        self.role = new_role;
        self.updated_at = Utc::now();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }

    #[allow(dead_code)]
    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Utc::now();
    }

    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn password_hash(&self) -> &PasswordHash {
        &self.password_hash
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn role(&self) -> &Role {
        &self.role
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn validate_name(name: &str) -> Result<(), DomainError> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 100 {
            return Err(DomainError::InvalidName);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(role: Role) -> User {
        let email = Email::new("test@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        User::new(email, hash, "Test User".into(), role).unwrap()
    }

    #[test]
    fn new_user_is_active_by_default() {
        let user = make_user(Role::User);
        assert!(user.is_active());
    }

    #[test]
    fn deactivate_sets_is_active_false() {
        let mut user = make_user(Role::User);
        user.deactivate();
        assert!(!user.is_active());
    }

    #[test]
    fn name_too_long_is_rejected() {
        let email = Email::new("a@b.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        let result = User::new(email, hash, "a".repeat(101), Role::User);
        assert_eq!(result.unwrap_err(), DomainError::InvalidName);
    }

    #[test]
    fn empty_name_is_rejected() {
        let email = Email::new("a@b.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        let result = User::new(email, hash, "   ".into(), Role::User);
        assert_eq!(result.unwrap_err(), DomainError::InvalidName);
    }

    #[test]
    fn admin_can_promote_to_user_not_admin() {
        let mut target = make_user(Role::User);
        let actor = make_user(Role::Admin);
        assert!(target.change_role(Role::User, &actor).is_ok());
        assert_eq!(
            target.change_role(Role::Admin, &actor).unwrap_err(),
            DomainError::InsufficientPermissions
        );
    }

    #[test]
    fn owner_can_promote_to_any_role() {
        let mut target = make_user(Role::User);
        let actor = make_user(Role::Owner);
        assert!(target.change_role(Role::Admin, &actor).is_ok());
        assert!(target.change_role(Role::Owner, &actor).is_ok());
    }

    #[test]
    fn new_user_has_correct_role() {
        let user = make_user(Role::Owner);
        assert_eq!(user.role(), &Role::Owner);

        let user = make_user(Role::Admin);
        assert_eq!(user.role(), &Role::Admin);
    }

    #[test]
    fn update_name_changes_name_and_updates_timestamp() {
        let mut user = make_user(Role::User);
        let before = user.updated_at();
        std::thread::sleep(std::time::Duration::from_millis(1));
        user.update_name("New Name".into()).unwrap();
        assert_eq!(user.name(), "New Name");
        assert!(user.updated_at() >= before);
    }

    #[test]
    fn update_password_changes_hash_and_updates_timestamp() {
        let mut user = make_user(Role::User);
        let before = user.updated_at();
        let new_hash = PasswordHash::from_phc_string("$argon2id$v=19$new_hash".into());
        std::thread::sleep(std::time::Duration::from_millis(1));
        user.update_password(new_hash.clone());
        assert_eq!(user.password_hash().value(), new_hash.value());
        assert!(user.updated_at() >= before);
    }

    #[test]
    fn activate_sets_is_active_true() {
        let mut user = make_user(Role::User);
        user.deactivate();
        assert!(!user.is_active());
        user.activate();
        assert!(user.is_active());
    }

    #[test]
    fn reconstitute_preserves_all_fields() {
        use crate::domain::value_objects::UserId;
        let id = UserId::new();
        let email = Email::new("reconstitute@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        let now = Utc::now();
        let user = User::reconstitute(UserSnapshot {
            id,
            email: email.clone(),
            password_hash: hash.clone(),
            name: "Reconstituted".into(),
            role: Role::Admin,
            is_active: false,
            created_at: now,
            updated_at: now,
        });
        assert_eq!(user.id(), id);
        assert_eq!(user.email(), &email);
        assert_eq!(user.name(), "Reconstituted");
        assert_eq!(user.role(), &Role::Admin);
        assert!(!user.is_active());
    }
}
