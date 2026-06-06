# Security

## Threat Model

This boilerplate is designed for multi-tenant web APIs exposed to the public internet. The primary threats addressed are:

- Credential stuffing and brute-force attacks
- Session hijacking and token theft
- Injection attacks (SQL, command)
- Information disclosure via error messages or logs
- Denial of service via resource exhaustion

---

## Password Hashing — Argon2id

All passwords are hashed using **Argon2id** via the `argon2` crate. Argon2id is the winner of the 2015 Password Hashing Competition and is resistant to both GPU-based attacks (time-hardness) and side-channel attacks (memory-hardness).

bcrypt and scrypt are not used.

### Parameters

```rust
Params::new(
    65536, // m_cost: 64 MB memory
    3,     // t_cost: 3 iterations
    4,     // p_cost: 4 parallel lanes
    None,
)
```

These parameters meet the OWASP minimum recommendations. Adjust upward based on your hardware profile and acceptable latency budget.

### Salt

A cryptographically random 16-byte salt is generated per-hash via `rand::thread_rng()`. The salt is embedded in the output string — never stored separately.

### Verification

Timing-safe comparison is handled by the `argon2` crate internally. Never implement your own comparison.

---

## Authentication

### Access Token (JWT HS256)

- Algorithm: HS256 (HMAC-SHA256)
- TTL: 15 minutes
- Claims: `sub` (user ID), `iat`, `exp`, `jti` (unique token ID)
- Storage: in-memory on the client — never in `localStorage` or cookies
- Validation: signature + expiry checked on every authenticated request

### Refresh Token

- Format: opaque UUID v4 (128 bits of entropy)
- Storage: server-side in Redis with TTL 7 days
- Transport: HttpOnly, Secure, SameSite=Strict cookie
- Rotation: a new refresh token is issued on every use; the old one is immediately invalidated
- Revocation: deleting the Redis key invalidates the session instantly

### Token Revocation

Access tokens cannot be revoked before expiry (stateless by design). The 15-minute TTL limits the exposure window. If immediate revocation is required, implement a short-lived Redis blocklist for `jti` values.

---

## Rate Limiting

Implemented as Axum middleware using a sliding window counter per IP address stored in Redis.

```
Default: 100 requests / 60 seconds per IP
Configurable via: RATE_LIMIT_RPS environment variable
```

Authentication endpoints (`/auth/login`, `/auth/register`) have a stricter independent limit to mitigate credential stuffing.

On limit exceeded, the server returns `429 Too Many Requests` with a `Retry-After` header.

---

## Security Headers

Applied globally via Axum middleware on every response:

| Header | Value |
|---|---|
| `Strict-Transport-Security` | `max-age=63072000; includeSubDomains; preload` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Content-Security-Policy` | `default-src 'none'` (API — no HTML served) |
| `Referrer-Policy` | `no-referrer` |
| `Permissions-Policy` | `geolocation=(), camera=(), microphone=()` |

---

## CORS

CORS is configured with an explicit allow-list. The wildcard `*` is never permitted in production.

```rust
CorsLayer::new()
    .allow_origin(config.allowed_origins)  // from environment variable
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    .allow_credentials(true)
```

---

## Input Validation

All inputs are validated at the HTTP boundary before reaching any use case. The `validator` crate enforces constraints on deserialized structs. Invalid input returns `400 Bad Request` with a structured error body — never a stack trace.

Domain-level invariants are re-enforced inside value objects regardless of what the HTTP layer does. The domain is the last line of defense.

---

## SQL Injection Prevention

All database queries use SQLx's compile-time checked parameterized queries. String interpolation into SQL is never used.

```rust
sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", email.value())
```

---

## Sensitive Data

- Passwords are never logged, never returned in API responses, and never stored in plain text
- Tokens are never logged
- Error responses to clients contain a message and an error code — never internal details, stack traces, or database errors
- `RUST_LOG` must never be set to `debug` or `trace` in production (would expose request bodies)

---

## Dependency Auditing

`cargo audit` runs on every CI push against the RustSec Advisory Database. Builds fail if a dependency has a known unfixed CVE.

```bash
cargo audit
```

Review `Cargo.lock` before deploying. Every transitive dependency is a potential attack surface.
