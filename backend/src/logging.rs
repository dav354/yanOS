//! Multi-output logging system for yanOS.
//!
//! Provides three independent logging outputs:
//! 1. CLI - Colorful, human-readable logs for terminal
//! 2. WebUI - Plain text logs streamed via EventBus
//! 3. OTLP - Structured logs/traces/metrics sent to collectors
//!
//! # Configuration
//! OTLP endpoints are configured independently in `config.json`:
//! - `telemetry.traces_endpoint` - Distributed tracing
//! - `telemetry.logs_endpoint` - Log export
//! - `telemetry.metrics_endpoint` - Metrics export
//!
//! Leave any endpoint empty to disable that export type.

use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use tracing::{info, warn};
use tracing_subscriber::{
    fmt::{self, time::ChronoLocal, writer::MakeWriter},
    layer::{Identity, Layer, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

use crate::config::TelemetryConfig;
use crate::error::AppError;
use crate::events::{EventBus, ExternalEvent};

/// Writer that publishes log lines to the EventBus for WebUI streaming.
#[derive(Clone)]
struct EventBusMakeWriter(EventBus);

/// Writer implementation for EventBus.
pub struct EventBusWriter(EventBus);

impl<'a> MakeWriter<'a> for EventBusMakeWriter {
    type Writer = EventBusWriter;

    fn make_writer(&'a self) -> Self::Writer {
        EventBusWriter(self.0.clone())
    }
}

impl std::io::Write for EventBusWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let line = String::from_utf8_lossy(buf).trim().to_string();
        if !line.is_empty() {
            self.0.publish(ExternalEvent::SystemLog { line });
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Initialize the multi-output tracing system.
///
/// Sets up three logging outputs:
/// 1. CLI with colors and pretty formatting
/// 2. WebUI with plain text via EventBus
/// 3. OTLP export (optional, based on config)
pub fn init_tracing(event_bus: EventBus, telemetry: &TelemetryConfig) -> Result<(), AppError> {
    // Create the env filter (respects RUST_LOG)
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| AppError::InternalServerError(format!("Failed to create EnvFilter: {e}")))?;

    // Layer 1: CLI with colors and pretty formatting
    let cli_layer = fmt::layer()
        .with_ansi(true) // Enable colors
        .with_target(true) // Show target (module path)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_timer(ChronoLocal::new("%H:%M:%S%.3f".to_string()))
        .pretty(); // Use pretty multi-line format

    // Layer 2: WebUI - plain text, no colors, streamed via EventBus
    let webui_layer = fmt::layer()
        .with_ansi(false) // No colors for web
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_timer(ChronoLocal::new("%H:%M:%S".to_string()))
        .compact() // Single line format
        .with_writer(EventBusMakeWriter(event_bus));

    // Build base subscriber with CLI and WebUI layers
    let base = Registry::default()
        .with(filter)
        .with(cli_layer)
        .with(webui_layer);

    // Layer 3: Tempo traces export (optional)
    let traces_layer = if let Some(endpoint) = telemetry.get_tempo_endpoint() {
        info!(target: "yanos::telemetry", endpoint = %endpoint, "Configuring Tempo traces export");

        let exporter = SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint.clone())
            .build()
            .map_err(|e| {
                AppError::InternalServerError(format!("Failed to build Tempo trace exporter: {e}"))
            })?;

        let resource = Resource::builder()
            .with_attributes([KeyValue::new("service.name", "yanos-backend")])
            .build();

        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource)
            .build();

        let tracer = provider.tracer("yanos-backend");
        global::set_tracer_provider(provider);

        Some(tracing_opentelemetry::layer().with_tracer(tracer).boxed())
    } else {
        None
    };

    // Log telemetry configuration status
    if telemetry.loki_endpoint.is_some() {
        // Note: Loki log export requires additional setup with opentelemetry-appender-tracing
        // For now, we just log that it's configured but not yet implemented
        warn!(
            target: "yanos::telemetry",
            endpoint = ?telemetry.loki_endpoint,
            "Loki endpoint configured (log export not yet implemented)"
        );
    }

    if telemetry.prometheus_endpoint.is_some() {
        // Note: Prometheus metrics are handled by the MetricsActor, not tracing
        info!(
            target: "yanos::telemetry",
            endpoint = ?telemetry.prometheus_endpoint,
            "Prometheus endpoint configured"
        );
    }

    if !telemetry.is_enabled() {
        warn!(
            target: "yanos::telemetry",
            "No telemetry receivers configured; export disabled"
        );
    }

    // Combine all layers and initialize
    let subscriber = base.with(traces_layer.unwrap_or_else(|| Identity::default().boxed()));

    subscriber.try_init().map_err(|e| {
        AppError::InternalServerError(format!("Failed to initialize tracing: {e}"))
    })?;

    Ok(())
}
