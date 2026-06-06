# ADR-0002: Use Argon2id for Password Hashing

**Date:** 2024-01-01  
**Status:** Accepted

---

## Context

Passwords must be stored as hashes that are computationally expensive to reverse. The choice of algorithm directly determines resistance to offline brute-force attacks after a database breach.

## Decision

**Argon2id** via the `argon2` crate with OWASP-recommended parameters (64 MB memory, 3 iterations, 4 lanes).

## Consequences

**Positive:**
- Argon2id won the 2015 Password Hashing Competition — the current state of the art.
- Memory-hardness (Argon2i component) resists side-channel attacks.
- Time-hardness (Argon2d component) resists GPU-based brute-force.
- Parameters are tunable without changing the storage format.

**Negative:**
- Higher memory and CPU cost per login than bcrypt — acceptable at the configured parameters (~100 ms on commodity hardware).

## Alternatives Considered

- **bcrypt** — widely deployed but has a 72-byte password limit and no memory-hardness; not recommended by OWASP for new systems.
- **scrypt** — memory-hard but more complex to parameterize correctly; Argon2id is preferred by OWASP since 2019.
- **PBKDF2** — FIPS-approved but not memory-hard; significantly weaker against GPU attacks.
