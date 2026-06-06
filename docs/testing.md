# Testing

## Philosophy

Tests are written before implementation (TDD). The test suite is organized in two strict tiers: unit tests that run in milliseconds with no external dependencies, and integration tests that run against real infrastructure.

The in-memory adapter exists precisely to make the entire business logic testable without Docker, a database, or any network call.

---

## Running Tests

```bash
# Unit tests only (fast, no external deps)
cargo test

# Integration tests (requires PostgreSQL and Redis)
cargo test --test integration

# Specific test
cargo test test_register_user_success

# With output
cargo test -- --nocapture

# Coverage (requires cargo-llvm-cov)
cargo llvm-cov --html
```

---

## Test Structure

```
src/
├── domain/
│   ├── value_objects/
│   │   ├── email.rs          # unit tests inline (#[cfg(test)])
│   │   └── password_hash.rs
│   └── entities/
│       └── user.rs           # entity invariant tests
│
├── application/
│   └── use_cases/
│       ├── register_user.rs  # use case tests with mock repositories
│       └── login_user.rs
│
tests/
├── integration/
│   ├── auth_flow.rs          # full HTTP flow against real DB
│   └── user_repository.rs   # adapter tests against real PostgreSQL
└── common/
    └── helpers.rs            # shared test setup
```

---

## Unit Tests

Unit tests live in `#[cfg(test)]` blocks inside the source file they test. They cover:

- Value object construction (valid and invalid inputs)
- Entity invariant enforcement
- Use case business logic (success and failure paths)

Repository dependencies are replaced with `mockall` mocks generated from the trait definition.

### Example — Value Object

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_email_is_accepted() {
        assert!(Email::new("user@example.com").is_ok());
    }

    #[test]
    fn email_without_at_sign_is_rejected() {
        assert!(matches!(
            Email::new("notanemail"),
            Err(DomainError::InvalidEmail)
        ));
    }
}
```

### Example — Use Case with Mock

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn register_user_succeeds_with_valid_input() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_by_email()
            .returning(|_| Ok(None)); // user does not exist yet
        mock_repo
            .expect_save()
            .returning(|_| Ok(()));

        let mock_hasher = MockPasswordHasher::new();
        // ... configure hasher mock

        let use_case = RegisterUser::new(Arc::new(mock_repo), Arc::new(mock_hasher));
        let result = use_case.execute(valid_input()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn register_user_fails_when_email_already_exists() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_by_email()
            .returning(|_| Ok(Some(existing_user())));

        // ... (no expect_save — it must not be called)

        let use_case = RegisterUser::new(Arc::new(mock_repo), Arc::new(mock_hasher()));
        let result = use_case.execute(valid_input()).await;

        assert!(matches!(result, Err(ApplicationError::EmailAlreadyExists)));
    }
}
```

---

## Integration Tests

Integration tests live in `tests/` and are compiled as separate crates. They run against a real PostgreSQL instance (configured via `TEST_DATABASE_URL`).

```rust
// tests/integration/auth_flow.rs

#[tokio::test]
async fn register_and_login_returns_valid_tokens() {
    let app = spawn_test_app().await;

    let res = app.post("/api/v1/auth/register")
        .json(&json!({ "email": "test@example.com", "password": "SecurePass123!" }))
        .send().await;

    assert_eq!(res.status(), 201);

    let res = app.post("/api/v1/auth/login")
        .json(&json!({ "email": "test@example.com", "password": "SecurePass123!" }))
        .send().await;

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await;
    assert!(body["access_token"].is_string());
}
```

Each integration test runs inside a transaction that is rolled back at the end — tests are isolated and the database is left clean.

---

## TDD Workflow

1. Write a failing test that describes the expected behavior
2. Run `cargo test` — confirm it fails for the right reason
3. Write the minimum implementation to make it pass
4. Run `cargo test` — confirm green
5. Refactor under green

Never write implementation code without a failing test first.

---

## Coverage Target

| Layer | Target |
|---|---|
| Domain (entities + value objects) | 100% |
| Application (use cases) | 100% |
| Infrastructure adapters | 80%+ |
| HTTP handlers | 70%+ (covered by integration tests) |
