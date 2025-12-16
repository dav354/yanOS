//! Network configuration API routes.
//!
//! Provides endpoints for reading and writing network configuration:
//! - GET /network/interfaces - List all interfaces with IP/link info
//! - GET /network/links - List physical links only
//! - GET /network/config - Get DNS, gateway, hostname
//! - POST /network/interface/:name/address - Set static IP
//! - POST /network/interface/:name/dhcp - Enable DHCP
//! - POST /network/config - Update DNS/gateway

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;

use crate::api::state::AppState;
use crate::core::{NetworkConfig, NetworkInterface, PhysicalLink};
use crate::error::AppError;

/// Response for successful operations.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Request to set a static IP address.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetAddressRequest {
    /// IP address (e.g., "192.168.1.10")
    pub address: String,
    /// Subnet prefix length (e.g., 24 for /24)
    pub prefix_len: u8,
}

/// Request to update system network config.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateConfigRequest {
    /// DNS servers to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_servers: Option<Vec<String>>,
    /// DNS search domains
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_search: Option<Vec<String>>,
    /// Default gateway
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/network/interfaces", get(list_interfaces))
        .route("/network/links", get(list_links))
        .route("/network/config", get(get_config).post(update_config))
        .route("/network/interface/{name}/address", post(set_address))
        .route("/network/interface/{name}/dhcp", post(set_dhcp))
}

/// List all network interfaces.
///
/// Returns combined data from dladm (physical links) and ipadm (IP addresses).
#[utoipa::path(
    get,
    path = "/api/v1/network/interfaces",
    tag = "network",
    responses(
        (status = 200, description = "Network interfaces", body = [NetworkInterface])
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(state))]
pub async fn list_interfaces(
    State(state): State<AppState>,
) -> Result<Json<Vec<NetworkInterface>>, AppError> {
    let interfaces = state.network_actor.list_interfaces().await?;
    Ok(Json(interfaces))
}

/// List physical network links.
///
/// Returns data from dladm including speed, MAC, and state.
#[utoipa::path(
    get,
    path = "/api/v1/network/links",
    tag = "network",
    responses(
        (status = 200, description = "Physical network links", body = [PhysicalLink])
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(state))]
pub async fn list_links(
    State(state): State<AppState>,
) -> Result<Json<Vec<PhysicalLink>>, AppError> {
    let links = state.network_actor.list_physical_links().await?;
    Ok(Json(links))
}

/// Get system network configuration.
///
/// Returns DNS servers, search domains, gateway, and hostname.
#[utoipa::path(
    get,
    path = "/api/v1/network/config",
    tag = "network",
    responses(
        (status = 200, description = "Network configuration", body = NetworkConfig)
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_config(State(state): State<AppState>) -> Result<Json<NetworkConfig>, AppError> {
    let config = state.network_actor.get_config().await?;
    Ok(Json(config))
}

/// Update system network configuration.
///
/// Updates DNS servers, search domains, and/or gateway.
#[utoipa::path(
    post,
    path = "/api/v1/network/config",
    tag = "network",
    request_body = UpdateConfigRequest,
    responses(
        (status = 200, description = "Configuration updated", body = SuccessResponse)
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(state))]
pub async fn update_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<SuccessResponse>, AppError> {
    // Update DNS if provided
    if req.dns_servers.is_some() || req.dns_search.is_some() {
        let current = state.network_actor.get_config().await?;
        let servers = req.dns_servers.unwrap_or(current.dns_servers);
        let search = req.dns_search.unwrap_or(current.dns_search);
        state.network_actor.set_dns(servers, search).await?;
    }

    // Update gateway if provided
    if let Some(gateway) = req.gateway {
        state.network_actor.set_gateway(gateway).await?;
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: Some("Network configuration updated".to_string()),
    }))
}

/// Set a static IP address on an interface.
#[utoipa::path(
    post,
    path = "/api/v1/network/interface/{name}/address",
    tag = "network",
    params(
        ("name" = String, Path, description = "Interface name (e.g., e1000g0)")
    ),
    request_body = SetAddressRequest,
    responses(
        (status = 200, description = "Address configured", body = SuccessResponse)
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(state))]
pub async fn set_address(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SetAddressRequest>,
) -> Result<Json<SuccessResponse>, AppError> {
    state
        .network_actor
        .set_static_address(name.clone(), req.address, req.prefix_len)
        .await?;

    Ok(Json(SuccessResponse {
        success: true,
        message: Some(format!("Static address configured on {}", name)),
    }))
}

/// Configure DHCP on an interface.
#[utoipa::path(
    post,
    path = "/api/v1/network/interface/{name}/dhcp",
    tag = "network",
    params(
        ("name" = String, Path, description = "Interface name (e.g., e1000g0)")
    ),
    responses(
        (status = 200, description = "DHCP configured", body = SuccessResponse)
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(state))]
pub async fn set_dhcp(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<SuccessResponse>, AppError> {
    state.network_actor.set_dhcp(name.clone()).await?;

    Ok(Json(SuccessResponse {
        success: true,
        message: Some(format!("DHCP configured on {}", name)),
    }))
}
