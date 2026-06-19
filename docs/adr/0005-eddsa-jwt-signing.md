# ADR-0005: EdDSA (Ed25519) for JWT Access Token Signing

**Date:** 2026-06-19  
**Status:** Accepted

---

## Context

[ADR-0003](0003-jwt-stateless-auth.md) established stateless JWT access tokens, originally signed with HS256 (HMAC-SHA256). HS256 is symmetric: the same secret signs and verifies tokens, so every service that needs to verify a token must hold the secret that can also forge one. That is a reasonable constraint for a single-service boilerplate, but it does not generalize cleanly to a distributed system where multiple services validate tokens issued by one authentication service.

## Decision

Sign access tokens with **EdDSA (Ed25519)** instead of HS256, using an asymmetric key pair:
- `JWT_PRIVATE_KEY_PATH` — Ed25519 private key (PKCS#8 PEM), used only by the service that issues tokens.
- `JWT_PUBLIC_KEY_PATH` — the matching public key, used to verify tokens. Safe to distribute to other services without granting them the ability to forge tokens.

Keys are generated with standard OpenSSL tooling:

```bash
openssl genpkey -algorithm ed25519 -out jwt_private.pem
openssl pkey -in jwt_private.pem -pubout -out jwt_public.pem
```

The `jsonwebtoken` crate's `rust_crypto` feature (backed by `ed25519-dalek`) provides EdDSA support without introducing a C dependency.

## Consequences

**Positive:**
- Verification no longer requires the signing secret — a future second service could hold only the public key and verify tokens independently.
- Ed25519 signatures are smaller and faster to verify than RSA, and avoid the timing-attack surface historically associated with some ECDSA implementations.
- No new runtime dependency: the JWT library already supported EdDSA, only a Cargo feature flag was needed.

**Negative:**
- Breaking change: tokens issued under HS256 cannot be verified by the new code and vice versa. There is no dual-algorithm transition period — this is a reference boilerplate, not a production system with live sessions to migrate.
- Key management is marginally more involved than a single secret string: two PEM files must be generated and distributed instead of one random value.

## Alternatives Considered

- **ES256 (ECDSA P-256)** — also asymmetric and widely supported, but slower to verify than Ed25519 and has a track record of subtle implementation bugs (e.g., nonce reuse) in some libraries across languages. Ed25519's design avoids that entire class of failure mode.
- **Keep HS256, rely on a shared secret distributed out-of-band** — simpler for a single service, but does not demonstrate the asymmetric pattern that becomes necessary once more than one service needs to verify tokens.
