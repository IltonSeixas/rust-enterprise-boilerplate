# Observability

## Overview

Observability is built into the boilerplate from the start using **OpenTelemetry** — the vendor-neutral standard. You can export to any compatible backend (Jaeger, Grafana Tempo, Honeycomb, Datadog, AWS X-Ray) by changing a single environment variable.

The three pillars — **traces**, **metrics**, and **logs** — are correlated by trace ID so you can move seamlessly from a high-level metric spike to the exact trace, then to the log lines of the failing request.

---

## Traces

Every HTTP request and gRPC call is automatically instrumented as a span. Use cases emit child spans so you can see exactly where time is spent.

### Setup

```rust
// infrastructure/telemetry/tracing.rs
let exporter = SpanExporter::builder()
    .with_tonic()
    .with_endpoint(otlp_endpoint)
    .build()?;

let resource = Resource::builder()
    .with_attribute(KeyValue::new(SERVICE_NAME, service_name.to_string()))
    .build();

let provider = SdkTracerProvider::builder()
    .with_batch_exporter(exporter)
    .with_resource(resource)
    .build();

opentelemetry::global::set_tracer_provider(provider.clone());
let otel_layer = tracing_opentelemetry::layer().with_tracer(provider.tracer("rust-enterprise-boilerplate"));
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

`infrastructure/telemetry/metrics.rs` installs a global Prometheus recorder via `metrics_exporter_prometheus`. Its handle is exposed at `GET /metrics` in the Prometheus exposition format (`text/plain; version=0.0.4`).

The boilerplate ships with the recorder wired end-to-end but does not emit any custom metrics yet — that is intentionally left to the application you build on top of it. Use the `metrics` crate's `counter!`, `histogram!`, and `gauge!` macros from any layer to record your own application metrics; they will be picked up by the same recorder and rendered at `/metrics` automatically.

### Scrape config (Prometheus)

```yaml
scrape_configs:
  - job_name: rust-api
    static_configs:
      - targets: ['localhost:8080']
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
| `OTLP_ENDPOINT` | `http://localhost:4317` | OTLP gRPC endpoint for the trace exporter |
| `RUST_LOG` | `info` | Log level filter |

---

## Local Development

Start a local Jaeger all-in-one instance to visualize traces:

```bash
docker compose up jaeger
```

Open `http://localhost:16686` to browse traces.

The `docker-compose.yml` in the repository root includes Jaeger, Prometheus, and Grafana pre-configured.
