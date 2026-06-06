# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| latest (`main`) | Yes |

---

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Please send a private disclosure to:

**Ilton Seixas** — [contact@iltonseixas.com](mailto:contact@iltonseixas.com)

Include in your report:
- A clear description of the vulnerability
- Steps to reproduce the issue
- The potential impact (what an attacker could achieve)
- If available, a suggested fix or mitigation

You will receive an acknowledgment within **72 hours** and a resolution timeline within **7 days**.

---

## Scope

The following are considered in scope:
- Authentication and session management flaws
- Authorization bypasses
- Injection vulnerabilities (SQL, command, etc.)
- Cryptographic weaknesses
- Information disclosure via API responses, logs, or error messages
- Dependency vulnerabilities with a direct exploitation path in this project

The following are out of scope:
- Vulnerabilities in the user's deployment environment or configuration
- Issues already known and tracked in public CVE databases for upstream dependencies

---

## Disclosure Policy

This project follows **coordinated disclosure**. Once a fix is available and deployed, the vulnerability will be publicly documented in the [CHANGELOG](./CHANGELOG.md) with appropriate credit to the reporter (unless anonymity is requested).
