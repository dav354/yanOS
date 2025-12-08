// backend/src/api.rs

use axum::{
    Json, Router,
    extract::{
        FromRef, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    middleware,
    routing::get,
};
use axum_csrf::CsrfConfig;
use serde_json::{Value, json};
use tower_sessions::Session;
use tracing::{info, instrument};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::adapters;
use crate::auth::{self, DynSessionStore};
use crate::error::AppError;
use crate::events::{EventBus, ExternalEvent};
use crate::tls;

/// The main struct for generating OpenAPI documentation.
#[derive(OpenApi)]
#[openapi(
    paths(
        api_status,
        healthz_handler,
        readyz_handler,
        auth::login_handler,
        system_info,
        list_network,
        list_packages,
    ),
    components(
        schemas(auth::LoginPayload)
    ),
    tags(
        (name = "zOS", description = "zOS Management API")
    ),
    info(
        title = "zOS API",
        version = "1.0.0",
        description = "API for managing the zOS Storage Appliance",
    )
)]
pub struct ApiDoc;

#[derive(Clone, Debug)]
pub struct AppState {
    pub csrf_config: CsrfConfig,
    pub session_store: DynSessionStore,
    pub tls_state: tls::TlsState,
    pub event_bus: EventBus,
    pub network_actor: crate::actors::NetworkActorHandle,
    pub pkg_actor: crate::actors::PkgActorHandle,
}

impl AppState {
    pub fn new(
        csrf_config: CsrfConfig,
        session_store: DynSessionStore,
        tls_state: tls::TlsState,
        event_bus: EventBus,
        network_actor: crate::actors::NetworkActorHandle,
        pkg_actor: crate::actors::PkgActorHandle,
    ) -> Self {
        Self {
            csrf_config,
            session_store,
            tls_state,
            event_bus,
            network_actor,
            pkg_actor,
        }
    }
}

impl FromRef<AppState> for CsrfConfig {
    fn from_ref(input: &AppState) -> CsrfConfig {
        input.csrf_config.clone()
    }
}

/// Creates the main API router, including the Swagger UI.
pub fn create_router(app_state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/events", axum::routing::get(stream_events))
        .route("/metrics/live", axum::routing::get(stream_metrics))
        .route_layer(middleware::from_fn(auth::auth_guard));

    let api_routes = Router::new()
        .route("/status", get(api_status))
        .route("/system/info", get(system_info))
        .route("/network/interfaces", get(list_network))
        .route("/pkg/list", get(list_packages))
        .merge(protected_routes);

    let app = Router::new()
        .route("/healthz", get(healthz_handler))
        .route("/readyz", get(readyz_handler))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", api_routes);

    app.with_state(app_state)
}

/// Liveness probe endpoint.
#[utoipa::path(
    get,
    path = "/healthz",
    responses(
        (status = 200, description = "Service is alive")
    )
)]
#[instrument]
async fn healthz_handler() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Readiness probe endpoint.
#[utoipa::path(
    get,
    path = "/readyz",
    responses(
        (status = 200, description = "Service is ready to accept traffic"),
        (status = 503, description = "Service is not ready")
    )
)]
#[instrument]
async fn readyz_handler(State(app_state): State<AppState>) -> Result<Json<Value>, AppError> {
    if !app_state.tls_state.is_ready() {
        return Err(AppError::ServiceUnavailable(
            "TLS configuration not ready".to_string(),
        ));
    }

    auth::session_store_healthcheck(&app_state.session_store).await?;

    Ok(Json(json!({ "status": "ready" })))
}

/// Get the current status of the API.
#[utoipa::path(
    get,
    path = "/api/v1/status",
    responses(
        (status = 200, description = "API is running", body = Value)
    )
)]
#[instrument]
async fn api_status(session: Session) -> Json<Value> {
    info!("Responding to API status check");
    let username: Option<String> = session.get("username").await.unwrap_or(None);
    Json(json!({
        "status": "ok",
        "user": username
    }))
}

/// Get basic system info.
#[utoipa::path(
    get,
    path = "/api/v1/system/info",
    responses(
        (status = 200, description = "System info", body = Value)
    )
)]
#[instrument]
async fn system_info() -> Result<Json<Value>, AppError> {
    let info = adapters::get_system_info().map_err(|e| {
        AppError::InternalServerError(format!("Failed to collect system info: {e}"))
    })?;
    Ok(Json(json!({
        "hostname": info.hostname,
        "kernel_version": info.kernel_version,
        "uptime": info.uptime
    })))
}

/// Stream external events to the UI via WebSocket.
#[instrument(skip(state, ws, session))]
async fn stream_events(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    session: Session,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let username: Option<String> = session.get("username").await.unwrap_or(None);
    if username.is_none() {
        return Err(AppError::Unauthorized(
            "Authentication required".to_string(),
        ));
    }

    let rx = state.event_bus.subscribe();
    Ok(ws.on_upgrade(move |socket| async move {
        handle_event_socket(socket, rx).await;
    }))
}

/// List network interfaces (via NetworkActor).
#[utoipa::path(
    get,
    path = "/api/v1/network/interfaces",
    responses(
        (status = 200, description = "Network interfaces", body = [Value])
    )
)]
#[instrument(skip(state))]
async fn list_network(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let interfaces = state.network_actor.list_interfaces().await?;
    Ok(Json(serde_json::to_value(&interfaces).unwrap_or_default()))
}

/// List installed packages (via PkgActor).
#[utoipa::path(
    get,
    path = "/api/v1/pkg/list",
    responses(
        (status = 200, description = "Package list", body = [Value])
    )
)]
#[instrument(skip(state))]
async fn list_packages(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let pkgs = state.pkg_actor.list().await?;
    Ok(Json(serde_json::to_value(&pkgs).unwrap_or_default()))
}

async fn handle_event_socket(
    stream: WebSocket,
    mut rx: tokio::sync::broadcast::Receiver<ExternalEvent>,
) {
    let mut socket = stream;
    while let Ok(event) = rx.recv().await {
        let payload = match event {
            ExternalEvent::ConfigChanged(path) => json!({
                "type": "config_changed",
                "path": path
            }),
        };
        if socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .is_err()
        {
            break;
        }
    }
}

/// Stream live metrics over WebSocket (mocked if kstat not available).
#[instrument(skip(ws, session))]
async fn stream_metrics(
    ws: WebSocketUpgrade,
    session: Session,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let username: Option<String> = session.get("username").await.unwrap_or(None);
    if username.is_none() {
        return Err(AppError::Unauthorized(
            "Authentication required".to_string(),
        ));
    }

    Ok(ws.on_upgrade(move |socket| async move {
        handle_metrics_socket(socket).await;
    }))
}

fn sample_metrics() -> serde_json::Value {
    // Try kstat first (illumos); fall back to /proc on dev hosts.
    if let Ok(output) = std::process::Command::new("kstat")
        .args(["-p", "cpu_stat:::idle", "-p", "cpu_stat:::user"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut idle: u64 = 0;
            let mut user: u64 = 0;
            for line in text.lines() {
                let mut parts = line.split_whitespace();
                if let (Some(_), Some(val)) = (parts.next(), parts.next()) {
                    if line.contains("idle") {
                        idle = val.parse().unwrap_or(0);
                    } else if line.contains("user") {
                        user = val.parse().unwrap_or(0);
                    }
                }
            }
            return json!({ "cpu_user": user, "cpu_idle": idle });
        }
    }

    // Fallback: load from /proc/stat (Linux) if present.
    if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
        if let Some(line) = stat.lines().find(|l| l.starts_with("cpu ")) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let user: u64 = parts[1].parse().unwrap_or(0);
                let idle: u64 = parts[4].parse().unwrap_or(0);
                return json!({ "cpu_user": user, "cpu_idle": idle });
            }
        }
    }

    json!({ "cpu_user": 0, "cpu_idle": 0 })
}

async fn handle_metrics_socket(mut socket: WebSocket) {
    loop {
        let payload = sample_metrics();
        if socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .is_err()
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
