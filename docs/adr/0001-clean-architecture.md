# ADR-0001: Adopt Clean Architecture with Hexagonal Ports & Adapters

**Date:** 2026-06-06  
**Status:** Accepted

---

## Context

A backend boilerplate needs to remain useful as requirements evolve. The most common source of pain in long-lived systems is coupling between business logic and infrastructure concerns — when swapping a database, adding a transport protocol, or testing a use case requires touching unrelated code.

## Decision

The project adopts **Clean Architecture** (Robert C. Martin) in its Hexagonal / Ports & Adapters form (Alistair Cockburn). The codebase is organized into four layers with a strict inward-only dependency rule:

1. **Domain** — entities, value objects, repository traits. No infrastructure or framework crates (no `tokio`, `axum`, `tonic`, `sqlx`, `redis`, `jsonwebtoken`, `argon2`); plain data/utility crates such as `serde`, `uuid`, and `chrono` are fine.
2. **Application** — use cases, input/output port traits. Imports domain only, plus the same data/utility crates — never the infrastructure crates listed above.
3. **Infrastructure** — adapters (PostgreSQL, Redis, Argon2). Implements application ports.
4. **Interfaces** — HTTP handlers, gRPC services. Calls application use cases.

This dependency rule is enforced automatically by `tests/architecture_test.rs`, which scans `use` statements in `src/domain/` and `src/application/` — see [ADR-0006](0006-architecture-layering-test.md).

## Consequences

**Positive:**
- Domain and application layers are testable without any infrastructure — unit tests run in milliseconds using mock adapters.
- Swapping infrastructure (e.g., replacing SQLx with SeaORM) requires touching only the adapter, not the use cases.
- The architecture is self-documenting: the module structure maps directly to the conceptual layers.
- The dependency rule is a compiled, automatically-enforced test rather than a convention that erodes silently over time.

**Negative:**
- More initial boilerplate than a flat structure — requires discipline to maintain boundaries.
- Indirection between layers can make call traces longer to follow in a debugger.

## Alternatives Considered

- **Flat package structure** — simple but becomes unmaintainable as the codebase grows; business logic leaks into handlers.
- **MVC** — familiar but conflates application logic with presentation concerns.
