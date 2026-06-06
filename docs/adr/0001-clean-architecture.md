# ADR-0001: Adopt Clean Architecture with Hexagonal Ports & Adapters

**Date:** 2026-06-06  
**Status:** Accepted

---

## Context

A backend boilerplate needs to remain useful as requirements evolve. The most common source of pain in long-lived systems is coupling between business logic and infrastructure concerns — when swapping a database, adding a transport protocol, or testing a use case requires touching unrelated code.

## Decision

The project adopts **Clean Architecture** (Robert C. Martin) in its Hexagonal / Ports & Adapters form (Alistair Cockburn). The codebase is organized into four layers with a strict inward-only dependency rule:

1. **Domain** — entities, value objects, repository traits. Zero external crate imports.
2. **Application** — use cases, input/output port traits. Imports domain only.
3. **Infrastructure** — adapters (PostgreSQL, Redis, Argon2). Implements application ports.
4. **Interfaces** — HTTP handlers, gRPC services. Calls application use cases.

## Consequences

**Positive:**
- Domain and application layers are testable without any infrastructure — unit tests run in milliseconds using mock adapters.
- Swapping infrastructure (e.g., replacing SQLx with SeaORM) requires touching only the adapter, not the use cases.
- The architecture is self-documenting: the module structure maps directly to the conceptual layers.

**Negative:**
- More initial boilerplate than a flat structure — requires discipline to maintain boundaries.
- Indirection between layers can make call traces longer to follow in a debugger.

## Alternatives Considered

- **Flat package structure** — simple but becomes unmaintainable as the codebase grows; business logic leaks into handlers.
- **MVC** — familiar but conflates application logic with presentation concerns.
