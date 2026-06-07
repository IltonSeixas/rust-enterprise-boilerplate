# Architecture

## Overview

This project implements Clean Architecture (also known as Hexagonal Architecture or Ports & Adapters) combined with Domain-Driven Design tactical patterns. The goal is a codebase where the business rules can be read, tested, and reasoned about without any knowledge of Axum, SQLx, or any other infrastructure library.

---

## Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        Interfaces                           │
│              (Axum HTTP handlers, tonic gRPC)               │
├─────────────────────────────────────────────────────────────┤
│                       Application                           │
│              (Use Cases, Input/Output Ports)                │
├─────────────────────────────────────────────────────────────┤
│                         Domain                              │
│          (Entities, Value Objects, Repository Traits)       │
├──────────────────────────┬──────────────────────────────────┤
│     Infrastructure       │         Infrastructure           │
│   (PostgreSQL adapter)   │      (In-Memory adapter)         │
└──────────────────────────┴──────────────────────────────────┘
```

**Dependency rule:** source code dependencies point inward only. The domain knows nothing about the layers outside it.

---

## Directory Structure

```
src/
├── domain/
│   ├── entities/
│   │   └── user.rs                       # User aggregate root
│   ├── value_objects/
│   │   ├── email.rs                      # Email — validated on construction
│   │   ├── password_hash.rs              # Opaque wrapper around hashed bytes
│   │   └── user_id.rs                    # UUID newtype
│   ├── repositories/
│   │   └── user_repository.rs            # Trait: the only contract infra must fulfill
│   └── errors.rs                         # DomainError enum (thiserror)
│
├── application/
│   ├── use_cases/
│   │   ├── register_user.rs
│   │   ├── login_user.rs
│   │   ├── refresh_token.rs
│   │   ├── get_user.rs
│   │   ├── update_profile.rs
│   │   └── change_password.rs
│   ├── ports/
│   │   ├── password_hasher.rs            # Trait: hash + verify
│   │   └── token_service.rs              # Trait: issue, validate and rotate JWTs
│   └── dtos/
│       ├── auth_dtos.rs
│       └── user_dtos.rs
│
├── infrastructure/
│   ├── persistence/
│   │   ├── in_memory_user_repository.rs  # Default: zero-config, runs immediately
│   │   └── postgres_user_repository.rs   # Behind the "postgres" Cargo feature
│   ├── security/
│   │   ├── argon2_hasher.rs
│   │   └── jwt_token_service.rs          # Issues/validates JWTs, stores refresh tokens in Redis
│   └── telemetry/
│       ├── tracing.rs                    # JSON logs + OTLP trace exporter
│       └── metrics.rs                    # Prometheus recorder, served at /metrics
│
├── interfaces/
│   ├── http/
│   │   ├── router.rs
│   │   ├── middleware/
│   │   │   ├── auth.rs                   # require_auth — validates bearer tokens
│   │   │   ├── cors.rs                   # Explicit origin allow-list
│   │   │   └── security_headers.rs       # X-Content-Type-Options, X-Frame-Options, HSTS, Referrer-Policy
│   │   └── handlers/
│   │       ├── auth_handler.rs
│   │       ├── user_handler.rs
│   │       ├── health_handler.rs
│   │       └── metrics_handler.rs
│   └── grpc/
│       ├── auth_service.rs
│       ├── user_service.rs
│       ├── interceptor.rs
│       └── error.rs
│
└── main.rs
```

---

## Domain Layer

### Entities

`User` is the aggregate root. It owns its invariants — you cannot construct a `User` with an empty name or an invalid email. Construction goes through associated functions that return `Result<User, DomainError>`, never through public field access.

### Value Objects

Value objects are immutable and self-validating. `Email::new("bad")` returns `Err(DomainError::InvalidEmail)`. Once constructed, a value object is always valid — there is no separate validation step.

```rust
pub struct Email(String);

impl Email {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        // validation logic
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
```

### Repository Traits

Repository traits define what the domain needs from persistence — not how it is implemented. The `async_trait` macro bridges async Rust with trait objects.

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
}
```

---

## Application Layer

Each use case is a struct that holds references to the ports it needs (via `Arc<dyn Trait>`). It has a single public `execute` method. Use cases contain zero infrastructure code — they call port traits and handle domain errors.

```rust
pub struct RegisterUser {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl RegisterUser {
    pub async fn execute(&self, input: RegisterInput) -> Result<(), ApplicationError> {
        // 1. validate input DTO
        // 2. check uniqueness via repository port
        // 3. hash password via hasher port
        // 4. construct domain entity
        // 5. persist via repository port
    }
}
```

---

## Infrastructure Layer

Infrastructure structs implement the domain/application traits. They are the only place where `sqlx`, `argon2`, `redis`, or any external crate is imported.

The in-memory adapter uses `tokio::sync::RwLock<HashMap<...>>` and is production-equivalent for the domain — it satisfies the same trait contract.

---

## Wiring (main.rs)

`main.rs` is the composition root. It reads configuration, builds adapters, injects them into use cases, and starts the server. It is the only place where concrete types are named.

```rust
let user_repo: Arc<dyn UserRepository> = match config.adapter {
    Adapter::Memory   => Arc::new(InMemoryUserRepository::new()),
    Adapter::Postgres => Arc::new(PostgresUserRepository::new(&pool)),
};

let register_user = RegisterUser::new(Arc::clone(&user_repo), Arc::clone(&hasher));
```
