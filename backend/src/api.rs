// backend/src/api.rs

use axum::{routing::get, Json, Router};
use axum_csrf::{CsrfConfig, CsrfToken};
use serde_json::{json, Value};
use tracing::{info, instrument};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::auth;

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

/// Creates the main API router, including the Swagger UI.
pub fn create_router(csrf_config: CsrfConfig) -> Router {
    let api_routes: Router<CsrfConfig> = Router::new().route("/status", get(api_status));

    Router::new()
        .route("/healthz", get(healthz_handler))
        .route("/readyz", get(readyz_handler))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", api_routes)
        .with_state(csrf_config)
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
        (status = 200, description = "Service is ready to accept traffic")
    )
)]
#[instrument]
async fn readyz_handler() -> Json<Value> {
    let cert_path = std::path::Path::new("/etc/opt/storage-os/tls/cert.pem");
    let key_path = std::path::Path::new("/etc/opt/storage-os/tls/key.pem");

    if cert_path.exists() && key_path.exists() {
        Json(json!({ "status": "ready" }))
    } else {
        Json(json!({ "status": "not_ready", "error": "TLS certificates missing" }))
    }
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
async fn api_status(token: CsrfToken) -> Json<Value> {
    info!("Responding to API status check");
    Json(json!({
        "status": "ok",
        "csrf_token": token.authenticity_token().unwrap_or_default()
    }))
}
