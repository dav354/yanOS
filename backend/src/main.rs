use std::{net::SocketAddr, path::Path};

use axum::{response::Redirect, routing::get_service};
use axum_csrf::CsrfLayer;
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use rustls::crypto::ring;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, Registry, layer::{Identity, Layer, SubscriberExt}, util::SubscriberInitExt};
use tracing_subscriber::fmt::writer::MakeWriter;

use yanos_backend::api::{self, AppState};
use yanos_backend::auth;
use yanos_backend::config::{AppConfig, DEFAULT_CONFIG_PATH};
use yanos_backend::error::AppError;
use yanos_backend::{actors, events::EventBus, tls, watchers};

#[derive(Clone)]
struct BusMakeWriter(EventBus);

struct BusWriter(EventBus);

impl<'a> MakeWriter<'a> for BusMakeWriter {
    type Writer = BusWriter;

    fn make_writer(&'a self) -> Self::Writer {
        BusWriter(self.0.clone())
    }
}

impl std::io::Write for BusWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let line = String::from_utf8_lossy(buf).trim().to_string();
        if !line.is_empty() {
            self.0.publish(yanos_backend::events::ExternalEvent::SystemLog { line });
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Initializes the tracing system with OpenTelemetry.
fn init_tracing(event_bus: EventBus, otlp_endpoint: Option<String>) -> Result<(), AppError> {
    // Use JSON formatting for structured logging as per Roadmap 1.1
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true).json();
    // Mirror logs into EventBus for UI consumption
    let bus_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .with_writer(BusMakeWriter(event_bus));

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| {
            AppError::InternalServerError(format!("Failed to initialize EnvFilter: {e}"))
        })?;

    let base_subscriber = Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .with(bus_layer);

    let telemetry_layer = if let Some(endpoint) = otlp_endpoint {
        let exporter = SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint.clone())
            .build()
            .map_err(|e| {
                AppError::InternalServerError(format!("Failed to build OTLP exporter: {e}"))
            })?;

        let resource = Resource::builder()
            .with_attributes([KeyValue::new("service.name", "yanos-backend")])
            .build();

        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource)
            .build();

        let tracer = provider.tracer("yanos-backend");
        let _ = global::set_tracer_provider(provider);

        Some(tracing_opentelemetry::layer().with_tracer(tracer).boxed())
    } else {
        tracing::warn!(
            target: "yanos::telemetry",
            "OTLP endpoint not configured; OpenTelemetry exporter disabled"
        );
        None
    };

    let subscriber = base_subscriber.with(
        telemetry_layer.unwrap_or_else(|| Identity::default().boxed()),
    );

    subscriber.try_init().map_err(|e| {
        AppError::InternalServerError(format!("Failed to initialize tracing: {e}"))
    })?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Ensure rustls has a crypto provider installed (ring feature).
    if let Err(e) = ring::default_provider().install_default() {
        return Err(AppError::InternalServerError(format!(
            "Failed to install rustls crypto provider: {e:?}"
        )));
    }

    let config = AppConfig::load(DEFAULT_CONFIG_PATH)?;
    let event_bus = EventBus::new(1024);
    init_tracing(event_bus.clone(), config.telemetry.otlp_endpoint.clone())?;
    let tls_state = tls::TlsState::load(Path::new(tls::DEFAULT_TLS_DIR))
        .await
        .map_err(AppError::from)?;
    tls_state.spawn_reload_task();

    // Keep watchers and actors alive for the process lifetime.
    let watched_paths: Vec<PathBuf> = vec![];
    let _config_watcher = watchers::start_filesystem_watcher(&watched_paths, event_bus.clone())
        .await
        .map_err(|e| AppError::InternalServerError(format!("Watcher failed: {e}")))?;

    // Start Axum server
    let _log_watcher = watchers::start_system_log_watcher(
        Path::new("/var/adm/messages"),
        event_bus.clone(),
    )?;

    let network_actor = actors::start_network_actor();
    let pkg_actor = actors::start_pkg_actor(event_bus.clone());
    let zfs_actor = actors::start_zfs_actor()?;
    let metrics_state = actors::start_metrics_actor()?;

    let session_store = auth::memory_store();
    let session_layer = auth::create_session_layer(session_store.clone());
    let csrf_config = auth::create_csrf_config();
    let app_state = AppState::new(
        csrf_config.clone(),
        session_store,
        tls_state.clone(),
        event_bus.clone(),
        network_actor.clone(),
        pkg_actor.clone(),
        zfs_actor,
        metrics_state,
        std::path::PathBuf::from(DEFAULT_CONFIG_PATH),
    );

    // Spawn daily package update check
    {
        let pkg_actor = app_state.pkg_actor.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
            // First tick completes immediately, but we might want to skip it if the actor does initial check.
            // However, actor does initial check on start, so we can just wait for next tick or let it double check.
            interval.tick().await; // skip first immediate tick
            
            loop {
                interval.tick().await;
                info!(target: "yanos::scheduler", "Triggering daily package update check");
                pkg_actor.check_updates().await;
            }
        });
    }

    let api_app = api::create_router();

    let api_app = auth::add_auth_routes(api_app)
        .layer(CsrfLayer::new(csrf_config))
        .layer(session_layer)
        .layer(CookieManagerLayer::new())
        .with_state(app_state.clone());

    let static_dir = std::env::var("YANOS_UI_DIR").unwrap_or_else(|_| "/opt/yanos/ui".to_string());
    let index_file = Path::new(&static_dir).join("index.html");
    let static_service = get_service(
        ServeDir::new(static_dir.clone()).not_found_service(ServeFile::new(index_file)),
    );

    let app = axum::Router::new()
        .merge(api_app)
        .fallback_service(static_service);

    let https_addr = SocketAddr::from(([0, 0, 0, 0], 8443));
    tokio::spawn(async {
        if let Err(err) = redirect_http_to_https().await {
            error!(target: "yanos::redirect", error = ?err, "HTTP redirect server error");
        }
    });

    info!(target: "yanos::main", "HTTPS server listening on https://{}", https_addr);
    info!(target: "yanos::main", "Swagger UI available at https://{}/swagger-ui", https_addr);

    axum_server::bind_rustls(https_addr, tls_state.config())
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(AppError::from)?;

    Ok(())
}

/// Spawns a separate server to redirect all HTTP traffic to HTTPS.
async fn redirect_http_to_https() -> Result<(), AppError> {
    let http_addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let redirect_app = axum::Router::new().fallback(
        |headers: axum::http::HeaderMap, uri: axum::http::Uri| async move {
            // Extract host from headers
            let host = headers
                .get(axum::http::header::HOST)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost");
            // Strip port from host if present, then add HTTPS port
            let hostname = host.split(':').next().unwrap_or("localhost");
            let new_uri = format!(
                "https://{}:8443{}",
                hostname,
                uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
            );
            Redirect::permanent(&new_uri)
        },
    );

    info!(target: "yanos::redirect", "Redirecting HTTP on {} to HTTPS on 8443", http_addr);
    let listener = TcpListener::bind(http_addr).await.map_err(|e| {
        AppError::InternalServerError(format!("Failed to bind redirect listener: {e}"))
    })?;

    axum::serve(listener, redirect_app.into_make_service())
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redirect server failed: {e}")))?;
    Ok(())
}
