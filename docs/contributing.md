# Contributing

Contributions are welcome. Please read this document before opening a pull request.

---

## Prerequisites

- Rust 1.78+ (`rustup update stable`)
- `cargo-llvm-cov` for coverage: `cargo install cargo-llvm-cov`
- `cargo-audit` for security auditing: `cargo install cargo-audit`
- Docker (for integration tests)

---

## Development Workflow

```bash
# Install dependencies and build
cargo build

# Run all unit tests
cargo test

# Run linter
cargo clippy -- -D warnings

# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt

# Run security audit
cargo audit

# Run integration tests (requires Docker)
cargo test --test integration
```

All of the above run automatically in CI on every pull request. A PR will not be merged if any of these steps fail.

---

## Code Standards

### Architecture

- Never import infrastructure packages from `domain/` or `application/`
- Every new use case must have a corresponding test file
- Every new value object must validate its invariants in the constructor and have tests for both valid and invalid inputs

### Style

- Follow `rustfmt` formatting — `cargo fmt` before committing
- Zero `clippy` warnings — run `cargo clippy -- -D warnings`
- No `unwrap()` or `expect()` in non-test code — use `?` or explicit error handling
- No `todo!()` or `unimplemented!()` in committed code
- No comments that explain *what* the code does — only *why* when non-obvious

### Tests

- New behavior requires a test written first (TDD)
- Mock repositories via `mockall` — never use real infrastructure in unit tests
- Integration tests must clean up after themselves (transactions rolled back)

---

## Pull Request Guidelines

1. Fork the repository and create a branch from `main`
2. Branch naming: `feat/short-description`, `fix/short-description`, `docs/short-description`
3. Keep each PR focused on a single concern
4. Include tests for every behavior change
5. Update relevant documentation in `docs/` if the change affects it
6. Ensure CI passes before requesting review

---

## Commit Convention

```
feat: add password reset use case
fix: correct argon2 salt length
docs: update security configuration reference
refactor: extract email validation into value object
test: add integration test for login flow
chore: update dependencies
```

---

## Reporting Security Vulnerabilities

Do **not** open a public GitHub issue for security vulnerabilities.

Send a private disclosure to [contact@iltonseixas.com](mailto:contact@iltonseixas.com) with:
- A description of the vulnerability
- Steps to reproduce
- Potential impact

You will receive a response within 72 hours.

---

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
