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
| `PORT` | No | `3000` | HTTP listen port |
| `GRPC_PORT` | No | `50051` | gRPC listen port |

### Persistence

| Variable | Required | Default | Description |
|---|---|---|---|
| `ADAPTER` | No | `memory` | Persistence adapter: `memory` or `postgres` |
| `DATABASE_URL` | If `postgres` | — | PostgreSQL connection string (`postgres://user:pass@host/db`) |
| `DATABASE_MAX_CONNECTIONS` | No | `10` | Connection pool size |
| `DATABASE_MIN_CONNECTIONS` | No | `1` | Minimum idle connections |
| `DATABASE_ACQUIRE_TIMEOUT_SECS` | No | `30` | Connection acquire timeout |

### Cache

| Variable | Required | Default | Description |
|---|---|---|---|
| `REDIS_URL` | If `postgres` | — | Redis connection string (`redis://host:port`) |

### Authentication

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | Yes | — | HS256 signing key — minimum 32 characters, use a random value |
| `JWT_ACCESS_TTL_SECS` | No | `900` | Access token TTL in seconds (15 min) |
| `JWT_REFRESH_TTL_SECS` | No | `604800` | Refresh token TTL in seconds (7 days) |

### Security

| Variable | Required | Default | Description |
|---|---|---|---|
| `ALLOWED_ORIGINS` | No | `http://localhost:*` | Comma-separated CORS allowed origins |
| `RATE_LIMIT_RPS` | No | `100` | Max requests per second per IP |
| `RATE_LIMIT_WINDOW_SECS` | No | `60` | Rate limit sliding window duration |

### Observability

| Variable | Required | Default | Description |
|---|---|---|---|
| `RUST_LOG` | No | `info` | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | No | `http://localhost:4317` | OTLP gRPC endpoint |
| `OTEL_SERVICE_NAME` | No | `rust-enterprise-boilerplate` | Service name in traces |
| `OTEL_SERVICE_VERSION` | No | — | Injected by CI from git tag |

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
