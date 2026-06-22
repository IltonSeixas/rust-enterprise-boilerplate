# rust-enterprise-boilerplate

[![CI](https://github.com/IltonSeixas/rust-enterprise-boilerplate/actions/workflows/ci.yml/badge.svg)](https://github.com/IltonSeixas/rust-enterprise-boilerplate/actions/workflows/ci.yml)
[![Docker](https://github.com/IltonSeixas/rust-enterprise-boilerplate/actions/workflows/docker.yml/badge.svg)](https://github.com/IltonSeixas/rust-enterprise-boilerplate/actions/workflows/docker.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)

Production-ready enterprise backend boilerplate in **Rust** — built on Clean Architecture, Domain-Driven Design, and Test-Driven Development. Zero configuration required to run; swap in a real database when you're ready.

---

## Philosophy

This boilerplate deliberately separates **what your system does** (domain logic) from **how it does it** (infrastructure). The core compiles and runs without a database, a message broker, or a cloud account. Every external dependency is behind an interface — swap the adapter, keep the domain.

---

## Architecture

```
src/
├── domain/               # Enterprise business rules — no dependencies
│   ├── entities/         # Aggregates and Entities
│   ├── value_objects/    # Immutable, self-validating values
│   ├── repositories/     # Port traits (interfaces)
│   └── errors.rs         # Domain error types
│
├── application/          # Use cases — depends only on domain
│   ├── use_cases/        # One struct per use case
│   ├── ports/            # Input/output port traits
│   └── dtos/             # Data transfer objects
│
├── infrastructure/       # Adapters — depends on application + domain
│   ├── persistence/      # in_memory_user_repository (default) + postgres_user_repository
│   ├── security/         # Argon2id hashing, JWT issuance/validation (Redis-backed refresh tokens)
│   └── telemetry/        # tracing (OTLP) and Prometheus metrics setup
│
├── interfaces/           # Entry points
│   ├── http/             # Axum REST handlers, middleware, routes
│   └── grpc/             # tonic gRPC services
│
└── main.rs               # Wiring: build dependency graph, start server
```

### Dependency rule

```
interfaces → application → domain
infrastructure → application → domain
```

Nothing in `domain/` or `application/` imports from `infrastructure/` or `interfaces/`. Ever. Enforced automatically by `tests/architecture_test.rs` (see [ADR-0006](docs/adr/0006-architecture-layering-test.md)) as part of the regular `cargo test` run.

---

## Stack

| Concern | Crate |
|---|---|
| Async runtime | `tokio` |
| HTTP framework | `axum` |
| gRPC | `tonic` + `prost` |
| Database (production) | `sqlx` (PostgreSQL) |
| Password hashing | `argon2` (Argon2id) |
| JWT | `jsonwebtoken` |
| Validation | self-validating domain value objects |
| Serialization | `serde` + `serde_json` |
| Edge security | `tower_governor` (rate limiting) + `tower-http` (CORS, security headers) |
| Observability | `opentelemetry` + `tracing` + `tracing-opentelemetry` |
| Error handling | `thiserror` + `anyhow` |
| Config | `config` + `dotenvy` |
| Testing | built-in + `mockall` |

---

## Getting Started

### Prerequisites

- Rust 1.96+ (`rustup update stable`)
- Optional for production: PostgreSQL 15+, Redis 7+

### Run immediately (in-memory, zero database)

```bash
git clone https://github.com/IltonSeixas/rust-enterprise-boilerplate
cd rust-enterprise-boilerplate
cp .env.example .env
openssl genpkey -algorithm ed25519 -out jwt_private.pem
openssl pkey -in jwt_private.pem -pubout -out jwt_public.pem
cargo run
```

The server starts on `http://localhost:8080` using the in-memory adapter. No database required.

### Run with PostgreSQL

```bash
cp .env.example .env
# Edit .env: set DATABASE_URL, JWT_PRIVATE_KEY_PATH, JWT_PUBLIC_KEY_PATH, etc.

cargo run --features postgres
```

---

## Security

### Password Hashing — Argon2id

All passwords are hashed using **Argon2id** — the winner of the Password Hashing Competition, resistant to both side-channel and GPU-based attacks. bcrypt is not used.

Parameters follow OWASP recommendations:
- Memory: 64 MB
- Iterations: 3
- Parallelism: 4

The `PasswordHasher` trait abstracts the algorithm — the domain never touches cryptographic primitives directly.

### Authentication Flow

- **Access token**: JWT EdDSA (Ed25519), TTL 15 min, stateless
- **Refresh token**: opaque UUID, stored server-side (Redis), TTL 7 days, rotated on every use
- **Revocation**: delete the Redis entry to immediately invalidate any session

### Security Middleware (applied globally)

- Rate limiting: per-IP token bucket via `tower_governor` (`RATE_LIMIT_PER_SECOND`/`RATE_LIMIT_BURST`, default 10 req/s with a burst of 20)
- Security headers: `X-Content-Type-Options`, `X-Frame-Options`, `Strict-Transport-Security`, `Referrer-Policy`
- CORS: explicit allow-list via `ALLOWED_ORIGINS`, never `*` — unlisted origins receive no CORS headers
- Input validation: self-validating domain value objects (`Email`, `PasswordHash`, ...) reject malformed input at construction time

### Audit Logging

Every identity- and access-sensitive use case (registration, login success/failure, password change, role change, token refresh) records an immutable `AuditEvent` through the `AuditPort` trait in `application/ports/`. The in-memory adapter is the zero-config default; the PostgreSQL adapter persists to a dedicated `audit_log` table and never fails the use case it observes, degrading gracefully if the audit store itself is unavailable.

---

## API

### REST — `http://localhost:8080`

| Method | Path | Description |
|---|---|---|
| `POST` | `/v1/auth/register` | Register a new user |
| `POST` | `/v1/auth/login` | Authenticate, receive tokens |
| `POST` | `/v1/auth/refresh` | Rotate refresh token |
| `GET` | `/v1/users/me` | Get authenticated user profile |
| `PUT` | `/v1/users/me` | Update authenticated user profile |
| `PUT` | `/v1/users/me/password` | Change authenticated user password |
| `GET` | `/v1/users/:id` | Get a user by id |
| `PUT` | `/v1/users/:id/role` | Change a user's role (Owner only, cannot change own role) |
| `GET` | `/health` | Liveness check |
| `GET` | `/ready` | Readiness check |
| `GET` | `/metrics` | Prometheus metrics |

### gRPC — `localhost:50051`

Proto definitions live in `proto/boilerplate.proto` and are compiled by `tonic-prost-build` from `build.rs` on every `cargo build` (the `protoc` binary is vendored via `protoc-bin-vendored`, so no system dependency is required).

| Service | RPC | Mirrors |
|---|---|---|
| `AuthService` | `Register`, `Login`, `RefreshToken` | `/v1/auth/*` |
| `UserService` | `GetMe`, `UpdateProfile`, `ChangePassword`, `ChangeRole` | `/v1/users/*` |

`UserService` RPCs require an `authorization: Bearer <access_token>` request metadata entry, validated the same way as the REST `require_auth` middleware (active-account check included).

---

## Testing

```bash
cargo test
```

### Structure

- **Unit tests**: co-located with source (`#[cfg(test)]` modules). Domain and use cases are tested in full isolation using `mockall`-generated mocks for repository, hasher and token-service ports; HTTP middleware (CORS, security headers) is exercised through `tower::ServiceExt::oneshot` against a minimal router.
- **Architecture tests**: `tests/architecture_test.rs` enforces the Clean Architecture dependency rule from [ADR-0001](docs/adr/0001-clean-architecture.md) at test time — see [ADR-0006](docs/adr/0006-architecture-layering-test.md). Runs as part of the regular `cargo test` step.

### TDD Approach

Use cases are written test-first:

1. Write a failing test against the use case interface
2. Implement the minimum domain logic to pass
3. Refactor under green

The in-memory adapter makes unit tests fast and deterministic — no test containers required for the core business logic.

---

## Observability

`infrastructure/telemetry` wires the three pillars on startup:

- **Traces**: a global `SdkTracerProvider` batches spans through an OTLP gRPC exporter (`opentelemetry-otlp` + `tracing-opentelemetry`), tagging them with the service name. `tracing::info!`/`#[tracing::instrument]` calls anywhere in the codebase become spans automatically.
- **Metrics**: `metrics_exporter_prometheus` installs a global recorder; the handle is exposed at `GET /metrics` in the Prometheus exposition format.
- **Logs**: structured JSON via `tracing-subscriber`, correlated with the active span through `with_current_span`/`with_span_list`.

Export traces to any OTLP-compatible backend (Jaeger, Grafana Tempo, Honeycomb, Datadog) by changing one variable:

```env
OTLP_ENDPOINT=http://localhost:4317
```

### Resilience

Redis calls made by `JwtTokenService` (refresh token issuance, validation, rotation and revocation) are wrapped in a `Closed → Open → Half-Open` circuit breaker (`infrastructure/resilience/circuit_breaker.rs`) combined with a retry policy. A transient Redis failure that succeeds on retry counts as a single success against the breaker's failure rate, rather than inflating it.

---

## Configuration

All configuration via environment variables (12-Factor). See `.env.example` for the full reference.

| Variable | Default | Description |
|---|---|---|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | HTTP port |
| `GRPC_PORT` | `50051` | gRPC port |
| `DATABASE_URL` | — | PostgreSQL connection string (only read when built with `--features postgres`) |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection string (refresh token storage) |
| `DB_POOL_MAX_CONNECTIONS` | `10` | Maximum number of pooled Postgres connections (only used with `--features postgres`) |
| `DB_POOL_MIN_CONNECTIONS` | `2` | Minimum number of idle Postgres connections kept open |
| `DB_POOL_CONNECT_TIMEOUT_MS` | `30000` | Max time to wait for a free connection from the pool, in milliseconds |
| `DB_POOL_IDLE_TIMEOUT_MS` | `600000` | Time before an idle connection above the minimum is closed, in milliseconds |
| `DB_POOL_MAX_LIFETIME_MS` | `1800000` | Max lifetime of a pooled connection before it is recycled, in milliseconds |
| `REDIS_CONNECT_TIMEOUT_MS` | `2000` | Max time to establish the Redis TCP connection, in milliseconds |
| `REDIS_COMMAND_TIMEOUT_MS` | `2000` | Max time to wait for a Redis command response, in milliseconds |
| `JWT_PRIVATE_KEY_PATH` | — | Path to the Ed25519 PEM private key used to sign access tokens |
| `JWT_PUBLIC_KEY_PATH` | — | Path to the Ed25519 PEM public key used to verify access tokens |
| `JWT_ACCESS_TTL_SECONDS` | `900` | Access token TTL, in seconds |
| `JWT_REFRESH_TTL_SECONDS` | `604800` | Refresh token TTL, in seconds |
| `ALLOWED_ORIGINS` | `http://localhost:3000` | Comma-separated CORS allow-list |
| `RATE_LIMIT_PER_SECOND` | `10` | Sustained requests/sec per IP |
| `RATE_LIMIT_BURST` | `20` | Burst capacity per IP |
| `OTLP_ENDPOINT` | `http://localhost:4317` | OTLP gRPC endpoint for traces |
| `RUST_LOG` | `info` | Log level (`tracing-subscriber` `EnvFilter` syntax) |

---

## Docker

```bash
# Build optimized multi-stage image (~20 MB final layer)
docker build -t rust-enterprise-boilerplate .

# Run (mount the key pair referenced by JWT_PRIVATE_KEY_PATH/JWT_PUBLIC_KEY_PATH in .env)
docker run -p 8080:8080 -p 50051:50051 --env-file .env \
  -v "$(pwd)/jwt_private.pem:/app/jwt_private.pem:ro" \
  -v "$(pwd)/jwt_public.pem:/app/jwt_public.pem:ro" \
  rust-enterprise-boilerplate
```

```bash
# Full stack: app + redis + jaeger + prometheus + grafana
# Requires jwt_private.pem/jwt_public.pem in the repo root — see Configuration above
openssl genpkey -algorithm ed25519 -out jwt_private.pem
openssl pkey -in jwt_private.pem -pubout -out jwt_public.pem
docker compose up
```

---

## CI/CD

GitHub Actions pipelines in `.github/workflows/`:

| Workflow | Trigger | Steps |
|---|---|---|
| `ci.yml` | push / PR | fmt, clippy, test, audit |
| `docker.yml` | push to `main` | build + push to GHCR |
| `release.yml` | tag `v*` | build binaries, create GitHub Release |

Security audit runs `cargo audit` on every push to catch known CVEs in dependencies.

---

## Plugging in a Real Database

The `UserRepository` trait in `domain/repositories/` is the only contract the domain cares about. To add a new adapter:

1. Implement the trait in `infrastructure/persistence/your_db/`
2. Wire it in `main.rs` behind a feature flag or config value
3. Run the integration test suite against your adapter

The in-memory adapter remains available for local development and CI unit tests.

---

## Author

**Ilton Seixas** — [contact@iltonseixas.com](mailto:contact@iltonseixas.com)

---

## Disclaimer

This boilerplate is provided **as-is**, for educational and reference purposes only.

**No warranty.** The author makes no representations or warranties of any kind, express or implied, regarding the correctness, completeness, reliability, suitability, or availability of this software for any purpose. Your use of this code is entirely at your own risk.

**No liability.** To the fullest extent permitted by applicable law, the author shall not be held liable for any direct, indirect, incidental, special, consequential, or punitive damages arising from the use or misuse of this software — including but not limited to data breaches, security incidents, financial loss, service downtime, or regulatory non-compliance.

**Misuse.** The author is not responsible for any unlawful, harmful, or unethical use of this codebase by any party.

**Security.** Security patterns and cryptographic implementations in this project follow industry best practices at the time of writing. However, the threat landscape evolves. You are solely responsible for auditing, hardening, and maintaining any system you build on top of this code.

> **Never blindly trust third-party code — including this project.**
> The author strongly recommends that you read and understand every line before deploying to production. Security-sensitive components (authentication, password hashing, token management, input validation) deserve particular scrutiny. No code review by a stranger on the internet replaces your own.

---

## License

MIT — Copyright (c) Ilton Seixas
