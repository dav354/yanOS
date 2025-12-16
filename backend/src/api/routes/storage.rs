//! Storage API routes for ZFS pool and dataset management.
//!
//! Provides endpoints for:
//! - Listing and getting pool information
//! - Listing and getting dataset information
//!
//! All endpoints require authentication via the auth_guard middleware.

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use tracing::instrument;

use crate::adapters::zfs::{DatasetInfo, PoolInfo};
use crate::api::AppState;
use crate::error::AppError;

/// List all ZFS pools.
#[utoipa::path(
    get,
    path = "/api/v1/storage/pools",
    tag = "storage",
    responses(
        (status = 200, description = "List of all pools", body = [PoolInfo]),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "ZFS service unavailable")
    )
)]
#[instrument(skip(state))]
pub async fn list_pools(State(state): State<AppState>) -> Result<Json<Vec<PoolInfo>>, AppError> {
    let pools = state.zfs_actor.list_pools().await?;
    Ok(Json(pools))
}

/// Get a specific pool by name.
#[utoipa::path(
    get,
    path = "/api/v1/storage/pools/{name}",
    tag = "storage",
    params(
        ("name" = String, Path, description = "Pool name")
    ),
    responses(
        (status = 200, description = "Pool information", body = PoolInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Pool not found"),
        (status = 503, description = "ZFS service unavailable")
    )
)]
#[instrument(skip(state))]
pub async fn get_pool(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PoolInfo>, AppError> {
    let pool = state.zfs_actor.get_pool(name).await?;
    Ok(Json(pool))
}

/// List all datasets in a pool.
#[utoipa::path(
    get,
    path = "/api/v1/storage/pools/{pool}/datasets",
    tag = "storage",
    params(
        ("pool" = String, Path, description = "Pool name")
    ),
    responses(
        (status = 200, description = "List of datasets in pool", body = [DatasetInfo]),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Pool not found"),
        (status = 503, description = "ZFS service unavailable")
    )
)]
#[instrument(skip(state))]
pub async fn list_datasets(
    State(state): State<AppState>,
    Path(pool): Path<String>,
) -> Result<Json<Vec<DatasetInfo>>, AppError> {
    let datasets = state.zfs_actor.list_datasets(pool).await?;
    Ok(Json(datasets))
}

/// Get a specific dataset by name.
#[utoipa::path(
    get,
    path = "/api/v1/storage/datasets/{name}",
    tag = "storage",
    params(
        ("name" = String, Path, description = "Full dataset name (pool/path)")
    ),
    responses(
        (status = 200, description = "Dataset information", body = DatasetInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Dataset not found"),
        (status = 503, description = "ZFS service unavailable")
    )
)]
#[instrument(skip(state))]
pub async fn get_dataset(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<DatasetInfo>, AppError> {
    let dataset = state.zfs_actor.get_dataset(name).await?;
    Ok(Json(dataset))
}

/// Create storage routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/storage/pools", get(list_pools))
        .route("/storage/pools/{name}", get(get_pool))
        .route("/storage/pools/{pool}/datasets", get(list_datasets))
        .route("/storage/datasets/{*name}", get(get_dataset))
}