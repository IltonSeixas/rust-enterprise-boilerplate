# ADR-0004: Use Axum as the HTTP Framework

**Date:** 2026-06-06  
**Status:** Accepted

---

## Context

Rust's async web ecosystem offers several mature options. The choice affects ergonomics, middleware composition, testability, and ecosystem compatibility.

## Decision

**Axum** (tokio-rs/axum) with **Tokio** as the async runtime.

## Consequences

**Positive:**
- Axum is maintained by the Tokio team — deep integration with the Tokio ecosystem (`tower`, `hyper`, `tracing`).
- Extractors and handlers are plain Rust functions — easy to test without spinning up a server.
- Tower middleware is composable and reusable across services.
- `tower-http` provides production-grade middleware (CORS, compression, tracing, rate limiting) out of the box.
- Strong type safety: incorrect handler signatures are compile-time errors.

**Negative:**
- Slightly steeper learning curve than Actix-web for developers new to `tower` and extractors.

## Alternatives Considered

- **Actix-web** — high performance and mature, but uses its own actor-based runtime; less aligned with the broader Tokio ecosystem and `tracing` integration.
- **Warp** — functional composition model is elegant but can produce complex type errors; less active development than Axum.
- **Rocket** — ergonomic but historically lagged on async support; nightly Rust dependency in older versions.
