//! Core data structures and types shared across the application.
//!
//! This module contains domain models that are used by adapters, actors, and API routes.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// System information including hostname, kernel version, and uptime.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemInfo {
    pub hostname: String,
    pub kernel_version: String,
    pub uptime: u64,
}

/// Physical network link information from dladm.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PhysicalLink {
    /// Link name (e.g., "e1000g0", "ixgbe0")
    pub name: String,
    /// Media type (e.g., "Ethernet")
    pub media: String,
    /// Link state ("up", "down", "unknown")
    pub state: String,
    /// Link speed in Mbps (e.g., 1000 for 1Gbps)
    pub speed: u64,
    /// Duplex mode ("full", "half", "unknown")
    pub duplex: String,
    /// MAC address
    pub mac: String,
    /// MTU size
    pub mtu: u32,
    /// User-defined friendly name/comment (stored in app config)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friendly_name: Option<String>,
}

/// IP address configuration for an interface from ipadm.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkAddress {
    /// Address object name (e.g., "e1000g0/v4", "e1000g0/dhcp")
    pub addrobj: String,
    /// Interface name (e.g., "e1000g0")
    pub interface: String,
    /// Address type ("static", "dhcp", "addrconf")
    pub addr_type: String,
    /// State ("ok", "tentative", "inaccessible", "disabled")
    pub state: String,
    /// IP address with prefix (e.g., "192.168.1.10/24")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

/// Combined network interface information.
/// Merges physical link data with IP address configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkInterface {
    /// Interface/link name
    pub name: String,
    /// Link state ("up", "down")
    pub state: String,
    /// Primary IP address (without prefix)
    pub address: String,
    /// Subnet prefix length (e.g., 24 for /24)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_len: Option<u8>,
    /// MAC address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    /// Link speed in Mbps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<u64>,
    /// MTU size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u32>,
    /// Address type ("static", "dhcp", "addrconf")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr_type: Option<String>,
    /// User-defined friendly name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friendly_name: Option<String>,
}

/// Network configuration for the entire system.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkConfig {
    /// DNS servers (from /etc/resolv.conf)
    pub dns_servers: Vec<String>,
    /// DNS search domains
    pub dns_search: Vec<String>,
    /// Default gateway (from /etc/defaultrouter)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    /// Hostname
    pub hostname: String,
}

/// Request to configure a static IP address on an interface.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetAddressRequest {
    /// Interface name (e.g., "e1000g0")
    pub interface: String,
    /// IP address (e.g., "192.168.1.10")
    pub address: String,
    /// Subnet prefix length (e.g., 24)
    pub prefix_len: u8,
}

/// Request to configure system network settings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetNetworkConfigRequest {
    /// DNS servers to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_servers: Option<Vec<String>>,
    /// DNS search domains to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_search: Option<Vec<String>>,
    /// Default gateway to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
}

/// Package information from pkg(1).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub build_time: String,
    pub status: String,
}
