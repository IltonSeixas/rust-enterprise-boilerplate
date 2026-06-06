# ADR-0003: Stateless JWT Access Tokens with Server-Side Refresh Tokens

**Date:** 2024-01-01  
**Status:** Accepted

---

## Context

Authentication requires a balance between statelessness (horizontal scalability) and revocability (security). Pure stateless JWTs cannot be revoked before expiry; pure server-side sessions require a shared store on every request.

## Decision

A **hybrid model**:
- **Access token**: stateless JWT HS256, TTL 15 minutes. Validated by signature and expiry only — no database lookup on every request.
- **Refresh token**: opaque UUID, stored server-side in Redis with TTL 7 days. Rotated on every use. Delivered via HttpOnly cookie.

## Consequences

**Positive:**
- Hot path (authenticated API calls) requires no database or Redis lookup — just cryptographic verification.
- Sessions can be revoked immediately by deleting the Redis entry (e.g., on logout or compromise detection).
- Refresh token rotation means a stolen refresh token is detected on next legitimate use (the old token is already gone).
- HttpOnly cookie prevents JavaScript-based token theft (XSS).

**Negative:**
- Access tokens cannot be revoked within their 15-minute window without a blocklist. Acceptable for most use cases; a `jti` Redis blocklist can be added if required.
- Redis becomes a required dependency when using server-side sessions (mitigated by the in-memory adapter for development).

## Alternatives Considered

- **Pure stateless JWT (long TTL)** — no revocation possible; unacceptable for a security-focused boilerplate.
- **Server-side sessions only** — requires a store lookup on every request; does not scale as cleanly.
- **OAuth2 / OIDC** — correct for multi-service or third-party auth; out of scope for a self-contained boilerplate.
