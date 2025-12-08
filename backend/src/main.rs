use axum::response::Redirect;
use axum_server::tls_rustls::RustlsConfig;
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use std::net::SocketAddr;
use tokio;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

mod adapters;
mod api;
mod auth;
mod core;
mod error;
mod tls;

use crate::error::AppError;

/// Initializes the tracing system with OpenTelemetry.
fn init_tracing() -> Result<(), AppError> {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .build()
        .map_err(|e| {
            AppError::InternalServerError(format!("Failed to build OTLP exporter: {e}"))
        })?;

    let resource = Resource::builder()
        .with_attributes([KeyValue::new("service.name", "zos-backend")])
        .build();

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("zos-backend");
    let _ = global::set_tracer_provider(provider);

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    // Use JSON formatting for structured logging as per Roadmap 1.1
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true).json();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| {
            AppError::InternalServerError(format!("Failed to initialize EnvFilter: {e}"))
        })?;

    Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .with(telemetry_layer)
        .try_init()
        .map_err(|e| AppError::InternalServerError(format!("Failed to initialize tracing: {e}")))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_tracing()?;
    tls::ensure_tls_certs_exist()?;

    let tls_config = RustlsConfig::from_pem_file(
        "/etc/opt/storage-os/tls/cert.pem",
        "/etc/opt/storage-os/tls/key.pem",
    )
    .await?;

    let session_layer = auth::create_session_layer();
    let app = api::create_router();

    let app = auth::add_auth_routes(app)
        .layer(session_layer)
        .layer(CookieManagerLayer::new());

    let https_addr = SocketAddr::from(([127, 0, 0, 1], 8443));
    tokio::spawn(async {
        if let Err(err) = redirect_http_to_https().await {
            error!(target: "zos::redirect", error = ?err, "HTTP redirect server error");
        }
    });

    info!(target: "zos::main", "🚀 HTTPS server listening on https://{}", https_addr);
    info!(target: "zos::main", "📚 Swagger UI available at https://{}/swagger-ui", https_addr);

    axum_server::bind_rustls(https_addr, tls_config)
        .serve(app.into_make_service())
        .await
        .map_err(AppError::from)?;

    Ok(())
}

/// Spawns a separate server to redirect all HTTP traffic to HTTPS.
async fn redirect_http_to_https() -> Result<(), AppError> {
    let http_addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let redirect_app = axum::Router::new().fallback(|uri: axum::http::Uri| async move {
        let new_uri = format!(
            "https://{}{}",
            uri.host().unwrap_or("localhost"),
            uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
        );
        Redirect::permanent(&new_uri)
    });

    info!(target: "zos::redirect", "Redirecting HTTP on {} to HTTPS on 8443", http_addr);
    let listener = TcpListener::bind(http_addr).await.map_err(|e| {
        AppError::InternalServerError(format!("Failed to bind redirect listener: {e}"))
    })?;

    axum::serve(listener, redirect_app.into_make_service())
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redirect server failed: {e}")))?;
    Ok(())
}
