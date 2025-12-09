use axum::{Json, Router, extract::State, routing::{get, post}};
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
        .route("/network/interfaces", get(list_network))
        .route("/pkg/list", get(list_packages))
        .route("/storage/dataset", post(create_dataset))
}

/// List network interfaces (via NetworkActor).
#[utoipa::path(
    get,
    path = "/api/v1/network/interfaces",
    tag = "resources",
    responses(
        (status = 200, description = "Network interfaces", body = [Value])
    )
)]
#[instrument(skip(state))]
pub async fn list_network(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let interfaces = state.network_actor.list_interfaces().await?;
    let value = serde_json::to_value(&interfaces)
        .map_err(|e| AppError::InternalServerError(format!("Failed to serialize interfaces: {e}")))?;
    Ok(Json(value))
}

/// List installed packages (via PkgActor).
#[utoipa::path(
    get,
    path = "/api/v1/pkg/list",
    tag = "resources",
    responses(
        (status = 200, description = "Package list", body = [Value])
    )
)]
#[instrument(skip(state))]
pub async fn list_packages(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let pkgs = state.pkg_actor.list().await?;
    let value = serde_json::to_value(&pkgs)
        .map_err(|e| AppError::InternalServerError(format!("Failed to serialize package list: {e}")))?;
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
    )
)]
#[instrument(skip(_state))]
pub async fn create_dataset(
    State(_state): State<AppState>,
    Json(_payload): Json<CreateDatasetRequest>
) -> Result<Json<CreateDatasetResponse>, AppError> {
    // Stub
    Ok(Json(CreateDatasetResponse { success: true }))
}
