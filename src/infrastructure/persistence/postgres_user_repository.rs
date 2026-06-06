#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::domain::{
    entities::{Role, User},
    errors::DomainError,
    repositories::UserRepository,
    value_objects::{Email, PasswordHash, UserId},
};

#[cfg(feature = "postgres")]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: String,
    name: String,
    role: String,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl TryFrom<UserRow> for User {
    type Error = DomainError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let email = Email::new(&row.email)?;
        let hash = PasswordHash::from_phc_string(row.password_hash);
        let role = match row.role.as_str() {
            "owner" => Role::Owner,
            "admin" => Role::Admin,
            _ => Role::User,
        };
        Ok(User::reconstitute(
            UserId::from_uuid(row.id),
            email,
            hash,
            row.name,
            role,
            row.is_active,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresUserRepository {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        let row = sqlx::query_as!(
            UserRow,
            r#"SELECT id, email, password_hash, name, role, is_active, created_at, updated_at
               FROM users WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let row = sqlx::query_as!(
            UserRow,
            r#"SELECT id, email, password_hash, name, role, is_active, created_at, updated_at
               FROM users WHERE email = $1"#,
            email.value()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        row.map(User::try_from).transpose()
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query!(
            r#"INSERT INTO users (id, email, password_hash, name, role, is_active, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (id) DO UPDATE SET
                 email = EXCLUDED.email,
                 password_hash = EXCLUDED.password_hash,
                 name = EXCLUDED.name,
                 role = EXCLUDED.role,
                 is_active = EXCLUDED.is_active,
                 updated_at = EXCLUDED.updated_at"#,
            user.id().value(),
            user.email().value(),
            user.password_hash().value(),
            user.name(),
            user.role().to_string(),
            user.is_active(),
            user.created_at(),
            user.updated_at(),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!("DELETE FROM users WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(())
    }

    async fn count(&self) -> Result<u64, DomainError> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(row.count.unwrap_or(0) as u64)
    }

    // Uses a partial unique index on role='owner' to guarantee atomicity at DB level.
    // The INSERT will fail with a unique violation if an owner already exists,
    // which we treat as Ok(false) rather than an error.
    async fn save_first_owner(&self, user: &User) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"INSERT INTO users (id, email, password_hash, name, role, is_active, created_at, updated_at)
               VALUES ($1, $2, $3, $4, 'owner', $5, $6, $7)"#,
            user.id().value(),
            user.email().value(),
            user.password_hash().value(),
            user.name(),
            user.is_active(),
            user.created_at(),
            user.updated_at(),
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(true),
            Err(sqlx::Error::Database(e)) if e.constraint() == Some("uq_users_owner_role") => {
                Ok(false)
            }
            Err(e) => Err(DomainError::Repository(e.to_string())),
        }
    }
}
