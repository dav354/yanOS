// backend/src/api.rs

use axum::{
    extract::{FromRef, State},
    routing::get,
    Json, Router,
};
use axum_csrf::{CsrfConfig, CsrfToken};
use serde_json::{json, Value};
use tower_sessions::Session;
use tracing::{info, instrument};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::auth::{self, DynSessionStore};
use crate::error::AppError;
use crate::tls;

/// The main struct for generating OpenAPI documentation.
#[derive(OpenApi)]
#[openapi(
    paths(
        api_status,
        healthz_handler,
        readyz_handler,
        auth::login_handler,
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
}

impl AppState {
    pub fn new(
        csrf_config: CsrfConfig,
        session_store: DynSessionStore,
        tls_state: tls::TlsState,
    ) -> Self {
        Self {
            csrf_config,
            session_store,
            tls_state,
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
    let api_routes = Router::new().route("/status", get(api_status));

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
#[instrument(skip(token))]
async fn api_status(token: CsrfToken, session: Session) -> Json<Value> {
    info!("Responding to API status check");
    let username: Option<String> = session.get("username").await.unwrap_or(None);
    Json(json!({
        "status": "ok",
        "csrf_token": token.authenticity_token().unwrap_or_default(),
        "user": username
    }))
}
