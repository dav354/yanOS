use axum::Router;
use axum::routing::get;
use axum::{Json, extract::State};
use serde_json::json;
use tracing::instrument;

use crate::api::state::AppState;
use crate::auth;
use crate::error::AppError;

pub fn routes() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}

/// Liveness probe endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/healthz",
    tag = "health",
    responses(
        (status = 200, description = "Service is alive")
    )
)]
#[instrument]
pub async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Readiness probe endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/readyz",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready to accept traffic"),
        (status = 503, description = "Service is not ready")
    )
)]
#[instrument]
pub async fn readyz(
    State(app_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !app_state.tls_state.is_ready() {
        return Err(AppError::ServiceUnavailable(
            "TLS configuration not ready".to_string(),
        ));
    }

    auth::session_store_healthcheck(&app_state.session_store).await?;

    Ok(Json(json!({ "status": "ready" })))
}
