# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Initial project structure: Clean Architecture + DDD layers
- In-memory user repository adapter (zero-config default)
- Argon2id password hashing via `argon2` crate
- JWT HS256 access token + opaque refresh token with Redis rotation
- Axum HTTP server with security middleware (rate limiting, CORS, security headers)
- tonic gRPC server with user service
- OpenTelemetry tracing, Prometheus metrics, structured JSON logs
- PostgreSQL adapter via SQLx
- Docker multi-stage image and docker-compose stack
- GitHub Actions CI (fmt, clippy, test, cargo-audit), Docker, and Release workflows
- Architecture documentation, ADRs, security policy

[Unreleased]: https://github.com/IltonSeixas/rust-enterprise-boilerplate/compare/HEAD
