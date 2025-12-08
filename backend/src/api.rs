// backend/src/api.rs

use axum::{routing::get, Json, Router};
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
pub fn create_router() -> Router {
    Router::new()
        .route("/healthz", get(healthz_handler))
        .route("/readyz", get(readyz_handler))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", Router::new().route("/status", get(api_status)))
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
    // In the future, this will check for TLS certs and session storage.
    Json(json!({ "status": "ready" }))
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
async fn api_status() -> Json<Value> {
    info!("Responding to API status check");
    Json(json!({ "status": "ok" }))
}
