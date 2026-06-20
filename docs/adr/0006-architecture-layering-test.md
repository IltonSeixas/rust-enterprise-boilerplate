# ADR-0006: Enforce the Layering Rule with an Automated Test

**Date:** 2026-06-19
**Status:** Accepted

---

## Context

[ADR-0001](0001-clean-architecture.md) defines a strict inward-only dependency rule between `domain/`, `application/`, `infrastructure/`, and `interfaces/`. Until now this rule was enforced only by code review and the contributor's own discipline — nothing in the build actually failed if a future change introduced, say, a `sqlx` import into `domain/`. Conventions enforced only by review erode the first time someone is in a hurry or unfamiliar with the codebase.

Auditing the existing code while writing this rule found that `domain/` and `application/` already depend on plain data/utility crates — `serde`, `uuid`, `chrono`, `thiserror`, `async_trait` — none of which couple the layers to a specific infrastructure choice. ADR-0001's original text ("zero external crate imports") did not match this reality and was stricter than necessary.

## Decision

Add `tests/architecture_test.rs`, a set of plain `#[test]` functions that scan `use` statements in `src/domain/` and `src/application/` for forbidden patterns. It runs as part of the existing `cargo test` step — no new dependency, no new CI step.

The rule distinguishes between two different concerns that the original ADR conflated:

1. **Data/utility crates** (`serde`, `uuid`, `chrono`, `thiserror`, `async_trait`) — these carry no infrastructure coupling; a value object serializing itself with `serde` does not know about HTTP, a database, or a message broker. Allowed in both `domain/` and `application/`.
2. **Infrastructure/framework crates** (`tokio`, `axum`, `tower*`, `tonic*`, `prost`, `sqlx`, `redis`, `argon2`, `password_hash`, `jsonwebtoken`, `config`, `dotenvy`, `tracing_subscriber`, `opentelemetry*`, `metrics*`) — these couple business logic to a specific runtime, transport, or persistence choice. Forbidden in both `domain/` and `application/`.

Four checks are encoded:
- `domain/` must not import any infrastructure crate from the list above.
- `domain/` must not depend on `application/`, `infrastructure/`, or `interfaces/` modules.
- `application/` must not import any infrastructure crate from the list above.
- `application/` must not depend on `infrastructure/` or `interfaces/` modules.

The test walks the real `.rs` files on disk and matches `use <crate>::` / `use <crate>;` statements — it has no awareness of comments or string literals containing crate names, but in practice false positives have not occurred, since none of the listed crate names appear as identifiers elsewhere in domain or application code.

## Consequences

**Positive:**
- A pull request that violates the layering rule now fails `cargo test` instead of relying on a reviewer noticing an import.
- The rule's text is the rule — no drift between what ADR-0001 says and what the codebase actually does.
- No new dependency: the test is plain Rust using `std::fs`.

**Negative:**
- Regex/string matching on source text is less precise than a real dependency graph — it would miss a violation hidden behind a macro-generated `use`, though none exist in this codebase today.
- The infrastructure-crate list must be kept in sync by hand as `Cargo.toml` changes; a newly added infrastructure crate is invisible to the test until added to the list.

## Alternatives Considered

- **Keep enforcing via code review only** — what ADR-0001 originally specified; demonstrated to drift from reality once domain/application adopted data/utility crates not mentioned in the ADR's stricter wording.
- **`cargo-deny` bans** — operates on the external dependency graph (which crates the whole binary pulls in), not on which *module* imports which crate; it cannot express "this crate may be used by `infrastructure/` but not by `domain/`" since both compile into the same binary.
- **Separate crates per layer (Cargo workspace)** — would let the compiler itself enforce the boundary (a crate that doesn't depend on `tokio` literally cannot use it), but is a much larger structural change than this boilerplate's single-crate layout warrants.
