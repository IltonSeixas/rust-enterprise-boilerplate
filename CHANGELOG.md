# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Initial project structure: Clean Architecture + DDD layers
- In-memory user repository adapter (zero-config default)
- Argon2id password hashing via `argon2` crate
- JWT access token + opaque refresh token with Redis rotation
- Axum HTTP server with security middleware (rate limiting, CORS, security headers)
- tonic gRPC server with user service
- OpenTelemetry tracing, Prometheus metrics, structured JSON logs
- PostgreSQL adapter via SQLx
- Docker multi-stage image and docker-compose stack
- GitHub Actions CI (fmt, clippy, test, cargo-audit), Docker, and Release workflows
- Architecture documentation, ADRs, security policy
- Code coverage reporting in CI
- `tests/architecture_test.rs` enforcing the Clean Architecture dependency rule from ADR-0001 at test time — see [ADR-0006](docs/adr/0006-architecture-layering-test.md)

### Changed
- **Breaking:** JWT access tokens are now signed with EdDSA (Ed25519) instead of HS256. `JWT_SECRET` is replaced by `JWT_PRIVATE_KEY_PATH`/`JWT_PUBLIC_KEY_PATH` — see [ADR-0005](docs/adr/0005-eddsa-jwt-signing.md). Tokens issued under the previous version are not valid under this one.

### Fixed
- Privilege escalation in role-change use case
- Refresh token rotation and role management edge cases

[Unreleased]: https://github.com/IltonSeixas/rust-enterprise-boilerplate/compare/HEAD
