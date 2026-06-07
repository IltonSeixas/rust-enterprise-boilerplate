# Testing

## Philosophy

Tests are written before implementation (TDD) and live alongside the code they test in `#[cfg(test)]` modules — there is no separate integration-test crate or tier. The in-memory adapter exists precisely to make the entire business logic testable without Docker, a database, or any network call.

---

## Running Tests

```bash
# Run the full suite
cargo test

# Specific test
cargo test register_user_succeeds_with_valid_input

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
└── interfaces/
    └── http/
        └── middleware/
            ├── cors.rs           # tower::ServiceExt::oneshot against a minimal router
            └── security_headers.rs
```

---

## Unit Tests

Tests live in `#[cfg(test)]` blocks inside the source file they exercise. They cover:

- Value object construction (valid and invalid inputs)
- Entity invariant enforcement
- Use case business logic (success and failure paths)
- HTTP middleware behavior, driven through `tower::ServiceExt::oneshot` against a minimal `Router` — no real network socket involved

Repository, hasher, and token-service dependencies are replaced with `mockall` mocks generated from the trait definitions.

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

## HTTP Middleware Tests

Middleware that depends on the HTTP layer (CORS, security headers) is exercised end-to-end with `tower::ServiceExt::oneshot` against a minimal `Router` — no real network socket, no test server process.

```rust
// interfaces/http/middleware/security_headers.rs

#[tokio::test]
async fn injects_expected_security_headers() {
    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .layer(middleware::from_fn(security_headers));

    let response = app
        .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let headers = response.headers();
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["x-frame-options"], "DENY");
}
```

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
| HTTP middleware | 80%+ |
