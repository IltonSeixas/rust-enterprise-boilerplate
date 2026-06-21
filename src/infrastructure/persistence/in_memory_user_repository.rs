use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::{
    entities::{Role, User},
    errors::DomainError,
    repositories::UserRepository,
    value_objects::Email,
};

pub struct InMemoryUserRepository {
    store: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        Ok(self.store.read().await.get(&id).cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let store = self.store.read().await;
        Ok(store.values().find(|u| u.email() == email).cloned())
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        self.store
            .write()
            .await
            .insert(user.id().value(), user.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.store.write().await.remove(&id);
        Ok(())
    }

    async fn count(&self) -> Result<u64, DomainError> {
        Ok(self.store.read().await.len() as u64)
    }

    async fn save_first_owner(&self, user: &User) -> Result<bool, DomainError> {
        let mut store = self.store.write().await;
        let owner_exists = store.values().any(|u| matches!(u.role(), Role::Owner));
        if owner_exists {
            return Ok(false);
        }
        store.insert(user.id().value(), user.clone());
        Ok(true)
    }

    async fn find_paginated(
        &self,
        offset: u64,
        limit: u64,
    ) -> Result<(Vec<User>, u64), DomainError> {
        let store = self.store.read().await;
        let mut users: Vec<User> = store.values().cloned().collect();
        users.sort_by(|a, b| {
            a.created_at()
                .cmp(&b.created_at())
                .then_with(|| a.id().value().cmp(&b.id().value()))
        });

        let total = users.len() as u64;
        let page = users
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        Ok((page, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::Role,
        value_objects::{Email, PasswordHash},
    };

    fn make_user(email_str: &str) -> User {
        let email = Email::new(email_str).unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        User::new(email, hash, "Test".into(), Role::User).unwrap()
    }

    #[tokio::test]
    async fn save_and_find_by_id() {
        let repo = InMemoryUserRepository::new();
        let user = make_user("a@b.com");
        let id = user.id().value();
        repo.save(&user).await.unwrap();
        let found = repo.find_by_id(id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn find_by_email_returns_correct_user() {
        let repo = InMemoryUserRepository::new();
        let user = make_user("unique@example.com");
        repo.save(&user).await.unwrap();
        let email = Email::new("unique@example.com").unwrap();
        let found = repo.find_by_email(&email).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn count_returns_number_of_users() {
        let repo = InMemoryUserRepository::new();
        assert_eq!(repo.count().await.unwrap(), 0);
        repo.save(&make_user("a@b.com")).await.unwrap();
        repo.save(&make_user("c@d.com")).await.unwrap();
        assert_eq!(repo.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn delete_removes_user() {
        let repo = InMemoryUserRepository::new();
        let user = make_user("del@example.com");
        let id = user.id().value();
        repo.save(&user).await.unwrap();
        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn save_first_owner_succeeds_when_no_owner_exists() {
        let repo = InMemoryUserRepository::new();
        let email = Email::new("owner@example.com").unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        let owner = User::new(email, hash, "Owner".into(), Role::Owner).unwrap();
        let saved = repo.save_first_owner(&owner).await.unwrap();
        assert!(saved);
        assert_eq!(repo.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn save_first_owner_rejects_second_owner() {
        let repo = InMemoryUserRepository::new();

        let email1 = Email::new("owner1@example.com").unwrap();
        let hash1 = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        let owner1 = User::new(email1, hash1, "Owner1".into(), Role::Owner).unwrap();
        let first = repo.save_first_owner(&owner1).await.unwrap();
        assert!(first);

        let email2 = Email::new("owner2@example.com").unwrap();
        let hash2 = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        let owner2 = User::new(email2, hash2, "Owner2".into(), Role::Owner).unwrap();
        let second = repo.save_first_owner(&owner2).await.unwrap();
        assert!(!second);

        assert_eq!(repo.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn find_paginated_returns_requested_slice_and_total() {
        let repo = InMemoryUserRepository::new();
        for i in 0..5 {
            repo.save(&make_user(&format!("user{i}@example.com")))
                .await
                .unwrap();
        }

        let (page, total) = repo.find_paginated(1, 2).await.unwrap();
        assert_eq!(total, 5);
        assert_eq!(page.len(), 2);
    }

    #[tokio::test]
    async fn find_paginated_returns_empty_page_past_the_end() {
        let repo = InMemoryUserRepository::new();
        repo.save(&make_user("solo@example.com")).await.unwrap();

        let (page, total) = repo.find_paginated(10, 20).await.unwrap();
        assert_eq!(total, 1);
        assert!(page.is_empty());
    }
}
