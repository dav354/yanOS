use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_sessions::Session;
use tracing::instrument;
use utoipa::ToSchema;

use crate::adapters;
use crate::api::state::AppState;
use crate::error::AppError;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct StatusResponse {
    pub status: String,
    pub user: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/status", get(get_status))
        .route("/system/info", get(system_info))
}

/// Get the current status of the API.
#[utoipa::path(
    get,
    path = "/api/v1/status",
    tag = "status",
    responses(
        (status = 200, description = "API is running", body = StatusResponse)
    )
)]
#[instrument(skip(session))]
pub async fn get_status(session: Session) -> Json<StatusResponse> {
    tracing::info!("Responding to API status check");
    let username: Option<String> = session.get("username").await.unwrap_or(None);
    Json(StatusResponse {
        status: "ok".to_string(),
        user: username,
    })
}

// Keeping legacy alias for mod.rs export if needed, or update mod.rs
pub use get_status as api_status;

/// Get basic system info.
#[utoipa::path(
    get,
    path = "/api/v1/system/info",
    tag = "status",
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
