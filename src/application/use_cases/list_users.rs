use std::sync::Arc;

use crate::{
    application::dtos::{ListUsersQuery, ListUsersResponse, PaginationMeta, UserResponse},
    domain::{errors::DomainError, repositories::UserRepository},
};

const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 100;

pub struct ListUsers {
    user_repo: Arc<dyn UserRepository>,
}

impl ListUsers {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    pub async fn execute(&self, query: ListUsersQuery) -> Result<ListUsersResponse, DomainError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query
            .page_size
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE);
        let offset = (page as u64 - 1) * page_size as u64;

        let (users, total_items) = self
            .user_repo
            .find_paginated(offset, page_size as u64)
            .await?;

        let total_pages = if total_items == 0 {
            0
        } else {
            ((total_items - 1) / page_size as u64 + 1) as u32
        };

        let items = users
            .into_iter()
            .map(|user| UserResponse {
                id: user.id().value(),
                email: user.email().value().to_owned(),
                name: user.name().to_owned(),
                role: user.role().clone(),
                is_active: user.is_active(),
                created_at: user.created_at(),
                updated_at: user.updated_at(),
            })
            .collect();

        Ok(ListUsersResponse {
            items,
            pagination: PaginationMeta {
                page,
                page_size,
                total_items,
                total_pages,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::{Role, User},
        value_objects::{Email, PasswordHash},
    };
    use mockall::mock;

    mock! {
        pub UserRepo {}
        #[async_trait::async_trait]
        impl UserRepository for UserRepo {
            async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<User>, DomainError>;
            async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError>;
            async fn save(&self, user: &User) -> Result<(), DomainError>;
            async fn delete(&self, id: uuid::Uuid) -> Result<(), DomainError>;
            async fn count(&self) -> Result<u64, DomainError>;
            async fn save_first_owner(&self, user: &User) -> Result<bool, DomainError>;
            async fn find_paginated(&self, offset: u64, limit: u64) -> Result<(Vec<User>, u64), DomainError>;
        }
    }

    fn make_user(email_str: &str) -> User {
        let email = Email::new(email_str).unwrap();
        let hash = PasswordHash::from_phc_string("$argon2id$v=19$...".into());
        User::new(email, hash, "Test User".into(), Role::User).unwrap()
    }

    #[tokio::test]
    async fn returns_items_and_pagination_metadata() {
        let mut repo = MockUserRepo::new();
        let users = vec![make_user("a@b.com"), make_user("c@d.com")];
        repo.expect_find_paginated()
            .withf(|offset, limit| *offset == 20 && *limit == 10)
            .returning(move |_, _| Ok((users.clone(), 25)));

        let uc = ListUsers::new(Arc::new(repo));
        let result = uc
            .execute(ListUsersQuery {
                page: Some(3),
                page_size: Some(10),
            })
            .await
            .unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.pagination.page, 3);
        assert_eq!(result.pagination.page_size, 10);
        assert_eq!(result.pagination.total_items, 25);
        assert_eq!(result.pagination.total_pages, 3);
    }

    #[tokio::test]
    async fn defaults_page_and_page_size_when_absent() {
        let mut repo = MockUserRepo::new();
        repo.expect_find_paginated()
            .withf(|offset, limit| *offset == 0 && *limit == 20)
            .returning(|_, _| Ok((vec![], 0)));

        let uc = ListUsers::new(Arc::new(repo));
        let result = uc
            .execute(ListUsersQuery {
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(result.pagination.page, 1);
        assert_eq!(result.pagination.page_size, 20);
        assert_eq!(result.pagination.total_pages, 0);
    }

    #[tokio::test]
    async fn clamps_page_size_to_maximum() {
        let mut repo = MockUserRepo::new();
        repo.expect_find_paginated()
            .withf(|offset, limit| *offset == 0 && *limit == 100)
            .returning(|_, _| Ok((vec![], 0)));

        let uc = ListUsers::new(Arc::new(repo));
        uc.execute(ListUsersQuery {
            page: Some(1),
            page_size: Some(1000),
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn clamps_page_to_minimum_of_one() {
        let mut repo = MockUserRepo::new();
        repo.expect_find_paginated()
            .withf(|offset, limit| *offset == 0 && *limit == 20)
            .returning(|_, _| Ok((vec![], 0)));

        let uc = ListUsers::new(Arc::new(repo));
        let result = uc
            .execute(ListUsersQuery {
                page: Some(0),
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(result.pagination.page, 1);
    }
}
