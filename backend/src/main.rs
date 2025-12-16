use std::{net::SocketAddr, path::Path, path::PathBuf};

use axum::{response::Redirect, routing::get_service};
use axum_csrf::CsrfLayer;
use rustls::crypto::ring;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};

use yanos_backend::api::{self, AppState};
use yanos_backend::auth;
use yanos_backend::config::{AppConfig, DEFAULT_CONFIG_PATH};
use yanos_backend::error::AppError;
use yanos_backend::logging;
use yanos_backend::{actors, events::EventBus, tls, watchers};

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
    logging::init_tracing(event_bus.clone(), &config.telemetry)?;
    let tls_state = tls::TlsState::load(Path::new(tls::DEFAULT_TLS_DIR))
        .await
        .map_err(AppError::from)?;
    tls_state.spawn_reload_task();

    // Keep watchers and actors alive for the process lifetime.
    let watched_paths: Vec<PathBuf> = vec![
        PathBuf::from("/etc/resolv.conf"),
        PathBuf::from("/etc/defaultrouter"),
        PathBuf::from("/etc/nodename"),
    ];
    let _config_watcher = watchers::start_filesystem_watcher(&watched_paths, event_bus.clone())
        .await
        .map_err(|e| AppError::InternalServerError(format!("Watcher failed: {e}")))?;

    // Network sysevent watcher
    let _network_watcher =
        watchers::start_network_event_watcher(event_bus.clone())?;

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
