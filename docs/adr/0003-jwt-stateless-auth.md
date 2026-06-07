# ADR-0003: Stateless JWT Access Tokens with Server-Side Refresh Tokens

**Date:** 2026-06-06  
**Status:** Accepted

---

## Context

Authentication requires a balance between statelessness (horizontal scalability) and revocability (security). Pure stateless JWTs cannot be revoked before expiry; pure server-side sessions require a shared store on every request.

## Decision

A **hybrid model**:
- **Access token**: stateless JWT HS256, TTL 15 minutes. Validated by signature and expiry only — no database lookup on every request.
- **Refresh token**: opaque UUID, stored server-side in Redis with TTL 7 days. Rotated on every use. Returned in the JSON response body alongside the access token; the client decides how to persist and resend it.

## Consequences

**Positive:**
- Hot path (authenticated API calls) requires no database or Redis lookup — just cryptographic verification.
- A compromised session can be invalidated immediately by deleting its refresh-token entry from Redis.
- Refresh token rotation means a stolen refresh token is detected on next legitimate use (the old token is already gone).

**Negative:**
- Access tokens cannot be revoked within their 15-minute window without an additional blocklist (e.g., a `jti`-keyed Redis set). Acceptable for most use cases given the short TTL.
- Redis becomes a required dependency for refresh-token storage (the in-memory adapter for the user repository does not remove this requirement).

## Alternatives Considered

- **Pure stateless JWT (long TTL)** — no revocation possible; unacceptable for a security-focused boilerplate.
- **Server-side sessions only** — requires a store lookup on every request; does not scale as cleanly.
- **OAuth2 / OIDC** — correct for multi-service or third-party auth; out of scope for a self-contained boilerplate.
