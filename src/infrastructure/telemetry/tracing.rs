use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{trace, runtime, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes structured JSON logging and an OTLP trace exporter, registering
/// the resulting `TracerProvider` globally so spans created anywhere in the
/// application are batched and shipped to the configured collector.
///
/// Call `opentelemetry::global::shutdown_tracer_provider` on shutdown to flush
/// pending spans before the process exits.
pub fn init_tracing(
    service_name: &str,
    otlp_endpoint: &str,
) -> Result<(), opentelemetry::trace::TraceError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint)
        .build_span_exporter()?;

    let provider = trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_config(trace::config().with_resource(Resource::new(vec![KeyValue::new(
            SERVICE_NAME,
            service_name.to_string(),
        )])))
        .build();

    opentelemetry::global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer("rust-enterprise-boilerplate");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_target(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    tracing::info!(service = service_name, "tracing initialized");

    Ok(())
}
