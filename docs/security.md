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

### Access Token (JWT EdDSA)

- Algorithm: EdDSA (Ed25519) — asymmetric signing via `jsonwebtoken`'s `rust_crypto` backend (`ed25519-dalek`)
- Keys: `JWT_PRIVATE_KEY_PATH` signs, `JWT_PUBLIC_KEY_PATH` verifies — only the signing service needs the private key, any service holding the public key can verify tokens independently
- TTL: 15 minutes (`JWT_ACCESS_TTL_SECONDS`)
- Claims: `sub` (user ID), `iat`, `exp`
- Transport: returned in the JSON response body (`access_token`); the client is responsible for storage and for sending it as `Authorization: Bearer <token>`
- Validation: signature + expiry checked on every authenticated request by the `require_auth` middleware (REST) and the gRPC interceptor

### Refresh Token

- Format: opaque UUID v4 (128 bits of entropy)
- Storage: server-side in Redis with TTL 7 days (`JWT_REFRESH_TTL_SECONDS`)
- Transport: returned in the JSON response body alongside the access token
- Rotation: a new refresh token is issued on every use; the old one is immediately invalidated
- Revocation: deleting the Redis key invalidates the session instantly

### Token Revocation

Access tokens cannot be revoked before expiry (stateless by design). The 15-minute TTL limits the exposure window. Refresh tokens, by contrast, are revocable instantly because they are stored server-side in Redis.

---

## Rate Limiting

Implemented as global Axum middleware (`tower_governor`) using a per-IP token-bucket (GCRA) algorithm — no external storage required.

```
Default: 10 requests/second sustained, burst capacity of 20, per IP
Configurable via: RATE_LIMIT_PER_SECOND / RATE_LIMIT_BURST environment variables
```

On limit exceeded, the server returns `429 Too Many Requests`.

---

## Security Headers

Applied globally via Axum middleware on every response:

| Header | Value |
|---|---|
| `Strict-Transport-Security` | `max-age=63072000; includeSubDomains` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |

---

## CORS

CORS is configured with an explicit allow-list. The wildcard `*` is never permitted.

```rust
CorsLayer::new()
    .allow_origin(AllowOrigin::list(origins))  // from ALLOWED_ORIGINS environment variable
    .allow_credentials(true)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
```

Origins not present in the allow-list receive no CORS headers at all — the browser blocks the response.

---

## Input Validation

All inputs are validated at the domain boundary through self-validating value objects (`Email::new`, `PasswordHash`, `UserId`, ...). Construction returns `Result<_, DomainError>` — a value object can never exist in an invalid state. Invalid input is translated to `400 Bad Request` with a structured error body — never a stack trace or internal detail.

Because validation lives in the domain rather than in a separate annotation layer, every entry point (REST, gRPC, future adapters) gets the same guarantees for free. The domain is the single source of truth and the last line of defense.

---

## SQL Injection Prevention

The PostgreSQL adapter (enabled via the `postgres` Cargo feature) uses SQLx's compile-time checked parameterized queries exclusively. String interpolation into SQL is never used.

```rust
sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", email.value())
```

The default build uses the in-memory adapter and never touches SQL at all.

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
