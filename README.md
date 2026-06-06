> **Work in progress** — this project is under active development and is not yet production-ready.

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
│   ├── persistence/
│   │   ├── in_memory/    # Default: zero-config, runs immediately
│   │   └── postgres/     # Production: SQLx + PostgreSQL
│   ├── security/         # Argon2id password hashing
│   ├── observability/    # OpenTelemetry traces, metrics, logs
│   └── cache/            # Redis adapter
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

Nothing in `domain/` or `application/` imports from `infrastructure/` or `interfaces/`. Ever.

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
| Validation | `validator` |
| Serialization | `serde` + `serde_json` |
| Observability | `opentelemetry` + `tracing` + `tracing-opentelemetry` |
| Error handling | `thiserror` + `anyhow` |
| Config | `config` + `dotenvy` |
| Testing | built-in + `mockall` |

---

## Getting Started

### Prerequisites

- Rust 1.78+ (`rustup update stable`)
- Optional for production: PostgreSQL 15+, Redis 7+

### Run immediately (in-memory, zero config)

```bash
git clone https://github.com/your-org/rust-enterprise-boilerplate
cd rust-enterprise-boilerplate
cargo run
```

The server starts on `http://localhost:3000` using the in-memory adapter. No database required.

### Run with PostgreSQL

```bash
cp .env.example .env
# Edit .env: set DATABASE_URL, JWT_SECRET, etc.

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

- **Access token**: JWT HS256, TTL 15 min, stateless
- **Refresh token**: opaque UUID, stored server-side (Redis), TTL 7 days, rotated on every use
- **Revocation**: delete the Redis entry to immediately invalidate any session

### Security Middleware (applied globally)

- Rate limiting: sliding window per IP
- Security headers: `X-Content-Type-Options`, `X-Frame-Options`, `Strict-Transport-Security`, `Content-Security-Policy`
- CORS: explicit allow-list, never `*` in production
- Input validation: `validator` crate on all DTOs at the HTTP boundary

---

## API

### REST — `http://localhost:3000`

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/v1/auth/register` | Register a new user |
| `POST` | `/api/v1/auth/login` | Authenticate, receive tokens |
| `POST` | `/api/v1/auth/refresh` | Rotate refresh token |
| `POST` | `/api/v1/auth/logout` | Revoke refresh token |
| `GET` | `/api/v1/users/me` | Get authenticated user profile |
| `GET` | `/health` | Health check |
| `GET` | `/metrics` | Prometheus metrics |

### gRPC — `localhost:50051`

Proto definitions live in `proto/`. Compile with:

```bash
cargo build  # tonic-build runs automatically via build.rs
```

---

## Testing

```bash
cargo test                    # unit tests (no external deps)
cargo test --test integration # integration tests (requires Postgres)
```

### Structure

- **Unit tests**: co-located with source (`#[cfg(test)]` blocks). Domain and use cases tested in full isolation using `mockall` mocks for repository ports.
- **Integration tests**: `tests/` directory. Spin up real adapters against a test database.

### TDD Approach

Use cases are written test-first:

1. Write a failing test against the use case interface
2. Implement the minimum domain logic to pass
3. Refactor under green

The in-memory adapter makes unit tests fast and deterministic — no test containers required for the core business logic.

---

## Observability

OpenTelemetry is wired in from the start:

- **Traces**: every HTTP request and gRPC call is a span; use cases emit child spans
- **Metrics**: request count, latency histograms, error rates exposed at `/metrics` (Prometheus format)
- **Logs**: structured JSON via `tracing`, correlated with trace IDs

Export to any OTLP-compatible backend (Jaeger, Grafana Tempo, Honeycomb, Datadog):

```env
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
```

---

## Configuration

All configuration via environment variables (12-Factor). See `.env.example` for the full reference.

| Variable | Default | Description |
|---|---|---|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `3000` | HTTP port |
| `GRPC_PORT` | `50051` | gRPC port |
| `DATABASE_URL` | — | PostgreSQL connection string |
| `REDIS_URL` | — | Redis connection string |
| `JWT_SECRET` | — | HS256 signing key (min 32 chars) |
| `JWT_ACCESS_TTL_SECS` | `900` | Access token TTL |
| `JWT_REFRESH_TTL_SECS` | `604800` | Refresh token TTL |
| `RATE_LIMIT_RPS` | `100` | Max requests/sec per IP |
| `RUST_LOG` | `info` | Log level |

---

## Docker

```bash
# Build optimized multi-stage image (~20 MB final layer)
docker build -t rust-enterprise-boilerplate .

# Run
docker run -p 3000:3000 -p 50051:50051 --env-file .env rust-enterprise-boilerplate
```

```bash
# Full stack: app + postgres + redis + jaeger
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
