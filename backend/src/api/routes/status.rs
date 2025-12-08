use axum::Json;
use serde_json::json;
use tower_sessions::Session;
use tracing::instrument;

use crate::adapters;
use crate::error::AppError;

/// Get the current status of the API.
#[utoipa::path(
    get,
    path = "/api/v1/status",
    responses(
        (status = 200, description = "API is running", body = serde_json::Value)
    )
)]
#[instrument]
pub async fn api_status(session: Session) -> Json<serde_json::Value> {
    tracing::info!("Responding to API status check");
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
        (status = 200, description = "System info", body = serde_json::Value)
    )
)]
#[instrument]
pub async fn system_info() -> Result<Json<serde_json::Value>, AppError> {
    let info = adapters::get_system_info().map_err(|e| {
        AppError::InternalServerError(format!("Failed to collect system info: {e}"))
    })?;
    Ok(Json(json!({
        "hostname": info.hostname,
        "kernel_version": info.kernel_version,
        "uptime": info.uptime
    })))
}
