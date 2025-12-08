use axum::{Json, extract::State};
use serde_json::Value;
use tracing::instrument;

use crate::api::state::AppState;
use crate::error::AppError;

/// List network interfaces (via NetworkActor).
#[utoipa::path(
    get,
    path = "/api/v1/network/interfaces",
    responses(
        (status = 200, description = "Network interfaces", body = [Value])
    )
)]
#[instrument(skip(state))]
pub async fn list_network(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
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
pub async fn list_packages(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let pkgs = state.pkg_actor.list().await?;
    Ok(Json(serde_json::to_value(&pkgs).unwrap_or_default()))
}
