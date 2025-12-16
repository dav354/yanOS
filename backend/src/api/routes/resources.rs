use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;
use utoipa::ToSchema;

use crate::api::state::AppState;
use crate::error::AppError;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct CreateDatasetRequest {
    pub name: String,
    pub pool: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateDatasetResponse {
    pub success: bool,
}

pub fn routes() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/pkg/list", get(list_packages))
        .route("/pkg/updates", get(list_updates))
        .route("/pkg/updates/check", post(check_updates))
        .route("/storage/dataset", post(create_dataset))
}

/// Trigger a manual check for package updates.
#[utoipa::path(
    post,
    path = "/api/v1/pkg/updates/check",
    tag = "resources",
    responses(
        (status = 202, description = "Update check started")
    )
)]
#[instrument(skip(state))]
pub async fn check_updates(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    state.pkg_actor.check_updates().await;
    axum::http::StatusCode::ACCEPTED
}

/// List available package updates.
#[utoipa::path(
    get,
    path = "/api/v1/pkg/updates",
    tag = "resources",
    responses(
        (status = 200, description = "Available updates", body = [Value])
    )
)]
#[instrument(skip(state))]
pub async fn list_updates(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let updates = state.pkg_actor.get_updates().await?;
    let value = serde_json::to_value(&updates).map_err(|e| {
        AppError::InternalServerError(format!("Failed to serialize updates: {e}"))
    })?;
    Ok(Json(value))
}

/// List installed packages (via PkgActor).
#[utoipa::path(
    get,
    path = "/api/v1/pkg/list",
    tag = "resources",
    responses(
        (status = 200, description = "Package list", body = [Value])
    ),
    security(
        ("basic_auth" = [])
    )
)]
#[instrument(skip(state))]
pub async fn list_packages(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let pkgs = state.pkg_actor.list().await?;
    let value = serde_json::to_value(&pkgs).map_err(|e| {
        AppError::InternalServerError(format!("Failed to serialize package list: {e}"))
    })?;
    Ok(Json(value))
}

/// Create a dataset (stub).
#[utoipa::path(
    post,
    path = "/api/v1/storage/dataset",
    tag = "resources",
    request_body = CreateDatasetRequest,
    responses(
        (status = 200, description = "Dataset created", body = CreateDatasetResponse)
    ),
    security(
        ("basic_auth" = [])
    )
)]
#[instrument(skip(_state))]
pub async fn create_dataset(
    State(_state): State<AppState>,
    Json(_payload): Json<CreateDatasetRequest>,
) -> Result<Json<CreateDatasetResponse>, AppError> {
    // Stub
    Ok(Json(CreateDatasetResponse { success: true }))
}
