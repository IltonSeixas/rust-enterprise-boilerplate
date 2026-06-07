# Configuration

All configuration is read from environment variables at startup. The application fails fast with a clear error message if any required variable is missing or invalid.

A `.env.example` file in the repository root lists every variable. Copy it to `.env` for local development.

```bash
cp .env.example .env
```

---

## Reference

### Server

| Variable | Required | Default | Description |
|---|---|---|---|
| `HOST` | No | `0.0.0.0` | Bind address |
| `PORT` | No | `8080` | HTTP listen port |
| `GRPC_PORT` | No | `50051` | gRPC listen port |

### Persistence

The persistence adapter is chosen at **compile time** via a Cargo feature flag, not a runtime variable:

```bash
cargo run                    # in-memory adapter (default, zero config)
cargo run --features postgres # PostgreSQL adapter
```

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Only when built with `--features postgres` | — | PostgreSQL connection string (`postgres://user:pass@host/db`) |

### Refresh Tokens

| Variable | Required | Default | Description |
|---|---|---|---|
| `REDIS_URL` | No | `redis://127.0.0.1:6379` | Redis connection string — backs refresh-token storage and rotation |

### Authentication

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | Yes | — | HS256 signing key — minimum 32 characters, use a random value |
| `JWT_ACCESS_TTL_SECONDS` | No | `900` | Access token TTL in seconds (15 min) |
| `JWT_REFRESH_TTL_SECONDS` | No | `604800` | Refresh token TTL in seconds (7 days) |

### Security

| Variable | Required | Default | Description |
|---|---|---|---|
| `ALLOWED_ORIGINS` | No | `http://localhost:3000` | Comma-separated CORS allow-list — the wildcard `*` is never honored |
| `RATE_LIMIT_PER_SECOND` | No | `10` | Sustained requests per second per IP (token-bucket refill rate) |
| `RATE_LIMIT_BURST` | No | `20` | Burst capacity per IP before throttling kicks in |

### Observability

| Variable | Required | Default | Description |
|---|---|---|---|
| `RUST_LOG` | No | `info` | Log level / `tracing-subscriber` `EnvFilter` directive (`error`, `warn`, `info`, `debug`, `trace`) |
| `OTLP_ENDPOINT` | No | `http://localhost:4317` | OTLP gRPC endpoint for the trace exporter |

---

## Production Checklist

Before deploying to production:

- [ ] `JWT_SECRET` is a random value of at least 32 characters — never reuse development values
- [ ] `DATABASE_URL` points to a production instance with TLS enabled (`sslmode=require`)
- [ ] `REDIS_URL` uses a password-protected Redis instance
- [ ] `ALLOWED_ORIGINS` lists only your actual frontend domains
- [ ] `RUST_LOG` is set to `info` or `warn` — never `debug` or `trace`
- [ ] `OTEL_EXPORTER_OTLP_ENDPOINT` points to your observability backend
- [ ] All secrets are injected via a secrets manager (HashiCorp Vault, AWS Secrets Manager, etc.) — never committed to source control
