# Observability

## Overview

Observability is built into the boilerplate from the start using **OpenTelemetry** — the vendor-neutral standard. You can export to any compatible backend (Jaeger, Grafana Tempo, Honeycomb, Datadog, AWS X-Ray) by changing a single environment variable.

The three pillars — **traces**, **metrics**, and **logs** — are correlated by trace ID so you can move seamlessly from a high-level metric spike to the exact trace, then to the log lines of the failing request.

---

## Traces

Every HTTP request and gRPC call is automatically instrumented as a span. Use cases emit child spans so you can see exactly where time is spent.

### Setup

```rust
// infrastructure/observability/setup.rs
let tracer = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(&config.otlp_endpoint),
    )
    .install_batch(opentelemetry_sdk::runtime::Tokio)?;

let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
```

### Manual spans in use cases

```rust
use tracing::instrument;

#[instrument(skip(self, input), fields(user.email = %input.email))]
pub async fn execute(&self, input: RegisterInput) -> Result<(), ApplicationError> {
    // ...
}
```

The `#[instrument]` macro creates a span for every call and attaches the listed fields. `skip` prevents sensitive data from being recorded.

---

## Metrics

Prometheus-format metrics are exposed at `GET /metrics`. The Axum middleware records request count and latency histograms automatically.

### Available metrics

| Metric | Type | Description |
|---|---|---|
| `http_requests_total` | Counter | Total HTTP requests by method, path, status |
| `http_request_duration_seconds` | Histogram | Request latency by method and path |
| `http_requests_in_flight` | Gauge | Currently active requests |
| `db_query_duration_seconds` | Histogram | Database query latency by operation |

### Scrape config (Prometheus)

```yaml
scrape_configs:
  - job_name: rust-api
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: /metrics
```

---

## Logs

Structured JSON logs via `tracing` + `tracing-subscriber`. Every log line includes the trace ID and span ID automatically, enabling correlation with distributed traces.

### Log format (production)

```json
{
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "INFO",
  "target": "boilerplate::application::use_cases::register_user",
  "message": "user registered",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "user.id": "01HN..."
}
```

### Log levels

| Level | Use |
|---|---|
| `ERROR` | Unrecoverable failures — always paged |
| `WARN` | Recoverable unexpected states |
| `INFO` | Business events (user registered, login succeeded) |
| `DEBUG` | Development only — never in production |
| `TRACE` | Never in production |

Set via `RUST_LOG=info` environment variable.

---

## Configuration

| Variable | Default | Description |
|---|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OTLP gRPC endpoint |
| `OTEL_SERVICE_NAME` | `rust-enterprise-boilerplate` | Service name in traces |
| `OTEL_SERVICE_VERSION` | — | Injected by CI from git tag |
| `RUST_LOG` | `info` | Log level filter |

---

## Local Development

Start a local Jaeger all-in-one instance to visualize traces:

```bash
docker compose up jaeger
```

Open `http://localhost:16686` to browse traces.

The `docker-compose.yml` in the repository root includes Jaeger, Prometheus, and Grafana pre-configured.
