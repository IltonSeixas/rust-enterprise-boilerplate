use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{ExporterBuildError, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes structured JSON logging and an OTLP trace exporter, registering
/// the resulting `SdkTracerProvider` globally so spans created anywhere in the
/// application are batched and shipped to the configured collector.
///
/// The returned provider must be shut down on process exit (via
/// `SdkTracerProvider::shutdown`) to flush pending spans.
pub fn init_tracing(
    service_name: &str,
    otlp_endpoint: &str,
) -> Result<SdkTracerProvider, ExporterBuildError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

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

    Ok(provider)
}
