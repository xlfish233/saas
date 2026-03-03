//! Telemetry and Tracing

use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .pretty();

    // Try to init OTLP if endpoint is available
    if let Ok(tracer) = init_otlp_tracer() {
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .try_init()
            .ok();
    } else {
        tracing_subscriber::registry()
            .with(fmt_layer)
            .try_init()
            .ok();
    }

    tracing::info!("Tracing initialized");
}

fn init_otlp_tracer() -> anyhow::Result<opentelemetry_sdk::trace::Tracer> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(format!("{}/v1/traces", otlp_endpoint))
        .build()?;

    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    Ok(tracer_provider.tracer("api-gateway"))
}
