//! Telemetry and tracing utilities

use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing with optional OTLP export
pub fn init_tracing(service_name: &'static str) {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);

    // Try to init OTLP if endpoint is available
    if let Ok(tracer) = init_otlp_tracer(service_name) {
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .try_init()
            .ok();
    } else {
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .try_init()
            .ok();
    }
}

fn init_otlp_tracer(
    service_name: &'static str,
) -> Result<opentelemetry_sdk::trace::Tracer, Box<dyn std::error::Error>> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(format!("{}/v1/traces", otlp_endpoint))
        .build()?;

    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name.to_string()),
        ]))
        .build();

    Ok(tracer_provider.tracer(service_name))
}

/// Shutdown tracing gracefully
pub fn shutdown_tracing() {
    opentelemetry::global::shutdown_tracer_provider();
}
