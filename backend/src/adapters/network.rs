//! Network adapter for illumos network configuration.
//!
//! This module provides functions to read and write network configuration
//! using illumos-native tools: dladm(8), ipadm(8), and system files.
//!
//! # Read Operations
//! - `get_physical_links()` - List physical network links via dladm
//! - `get_network_addresses()` - List IP addresses via ipadm
//! - `get_network_interfaces()` - Combined view merging dladm + ipadm data
//! - `get_network_config()` - DNS, gateway, hostname from system files
//!
//! # Write Operations
//! - `set_static_address()` - Configure static IP via ipadm
//! - `delete_address()` - Remove an address object via ipadm
//! - `set_dns_servers()` - Write /etc/resolv.conf
//! - `set_gateway()` - Write /etc/defaultrouter

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::process::Command;

use tracing::{debug, warn};

use crate::core::{NetworkAddress, NetworkConfig, NetworkInterface, PhysicalLink};
use crate::error::AppError;

/// Get physical network links via `dladm show-phys`.
///
/// Returns information about physical NICs including speed, MAC, and state.
pub fn get_physical_links() -> Result<Vec<PhysicalLink>, AppError> {
    debug!(target: "yanos::network", "Querying physical links via dladm show-phys");

    let output = Command::new("dladm")
        .args(["show-phys", "-p", "-o", "link,media,state,speed,duplex,mtu"])
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to run dladm: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!(target: "yanos::network", stderr = %stderr, "dladm show-phys failed");
        return Err(AppError::InternalServerError(format!(
            "dladm show-phys failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut links = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 6 {
            let name = parts[0].to_string();
            let media = parts[1].to_string();
            let state = parts[2].to_lowercase();
            let speed: u64 = parts[3].parse().unwrap_or(0);
            let duplex = parts[4].to_lowercase();
            let mtu: u32 = parts[5].parse().unwrap_or(1500);

            // Get MAC address separately
            let mac = get_link_mac(&name).unwrap_or_default();

            debug!(
                target: "yanos::network",
                link = %name,
                state = %state,
                speed,
                mac = %mac,
                "Found physical link"
            );

            links.push(PhysicalLink {
                name,
                media,
                state,
                speed,
                duplex,
                mac,
                mtu,
                friendly_name: None,
            });
        }
    }

    debug!(target: "yanos::network", count = links.len(), "Physical links query complete");
    Ok(links)
}

/// Get MAC address for a link via `dladm show-phys -m`.
fn get_link_mac(link: &str) -> Result<String, AppError> {
    let output = Command::new("dladm")
        .args(["show-phys", "-m", "-p", "-o", "address", link])
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to get MAC: {e}")))?;

    if output.status.success() {
        let mac = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(mac)
    } else {
        Ok(String::new())
    }
}

/// Get IP addresses via `ipadm show-addr`.
///
/// Returns all configured IP address objects with their state and type.
pub fn get_network_addresses() -> Result<Vec<NetworkAddress>, AppError> {
    debug!(target: "yanos::network", "Querying IP addresses via ipadm show-addr");

    let output = Command::new("ipadm")
        .args(["show-addr", "-p", "-o", "addrobj,type,state,addr"])
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to run ipadm: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!(target: "yanos::network", stderr = %stderr, "ipadm show-addr failed");
        return Err(AppError::InternalServerError(format!(
            "ipadm show-addr failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut addresses = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            let addrobj = parts[0].to_string();
            let addr_type = parts[1].to_lowercase();
            let state = parts[2].to_lowercase();
            let address = if parts.len() > 3 && !parts[3].is_empty() {
                Some(parts[3].to_string())
            } else {
                None
            };

            // Extract interface name from addrobj (e.g., "e1000g0/v4" -> "e1000g0")
            let interface = addrobj.split('/').next().unwrap_or(&addrobj).to_string();

            // Skip loopback
            if interface.starts_with("lo") {
                continue;
            }

            debug!(
                target: "yanos::network",
                addrobj = %addrobj,
                addr_type = %addr_type,
                state = %state,
                address = ?address,
                "Found address object"
            );

            addresses.push(NetworkAddress {
                addrobj,
                interface,
                addr_type,
                state,
                address,
            });
        }
    }

    debug!(target: "yanos::network", count = addresses.len(), "Address query complete");
    Ok(addresses)
}

/// Get combined network interface information.
///
/// Merges physical link data from dladm with IP addresses from ipadm.
pub fn get_network_interfaces() -> Result<Vec<NetworkInterface>, AppError> {
    debug!(target: "yanos::network", "Building combined interface list");

    // Get physical links
    let links = get_physical_links().unwrap_or_default();
    let link_map: HashMap<String, PhysicalLink> =
        links.into_iter().map(|l| (l.name.clone(), l)).collect();

    debug!(target: "yanos::network", physical_links = link_map.len(), "Got physical links");

    // Get IP addresses
    let addresses = get_network_addresses()?;

    // Group addresses by interface
    let mut iface_addrs: HashMap<String, Vec<NetworkAddress>> = HashMap::new();
    for addr in addresses {
        iface_addrs
            .entry(addr.interface.clone())
            .or_default()
            .push(addr);
    }

    let mut interfaces = Vec::new();

    // Build interfaces from addresses
    for (iface_name, addrs) in iface_addrs {
        // Find primary IPv4 address (prefer static, then dhcp)
        let primary = addrs
            .iter()
            .find(|a| a.addr_type == "static" && a.address.as_ref().is_some_and(|ip| !ip.contains(':')))
            .or_else(|| addrs.iter().find(|a| a.addr_type == "dhcp"))
            .or_else(|| addrs.iter().find(|a| a.address.is_some() && !a.address.as_ref().unwrap().contains(':')));

        let (address, prefix_len, addr_type, state) = if let Some(addr) = primary {
            let (ip, prefix) = parse_address_prefix(addr.address.as_deref().unwrap_or(""));
            (ip, prefix, Some(addr.addr_type.clone()), addr.state.clone())
        } else {
            (String::new(), None, None, "down".to_string())
        };

        // Get link info
        let link = link_map.get(&iface_name);

        interfaces.push(NetworkInterface {
            name: iface_name,
            state: link.map_or(state.clone(), |l| l.state.clone()),
            address,
            prefix_len,
            mac: link.map(|l| l.mac.clone()),
            speed: link.map(|l| l.speed),
            mtu: link.map(|l| l.mtu),
            addr_type,
            friendly_name: None,
        });
    }

    // Add physical links that have no IP addresses
    for (name, link) in &link_map {
        if !interfaces.iter().any(|i| &i.name == name) {
            interfaces.push(NetworkInterface {
                name: name.clone(),
                state: link.state.clone(),
                address: String::new(),
                prefix_len: None,
                mac: Some(link.mac.clone()),
                speed: Some(link.speed),
                mtu: Some(link.mtu),
                addr_type: None,
                friendly_name: None,
            });
        }
    }

    // Sort by name
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));

    debug!(target: "yanos::network", count = interfaces.len(), "Combined interface list complete");
    Ok(interfaces)
}

/// Parse an address like "192.168.1.10/24" into (ip, prefix_len).
fn parse_address_prefix(addr: &str) -> (String, Option<u8>) {
    if let Some((ip, prefix)) = addr.split_once('/') {
        let prefix_len = prefix.parse().ok();
        (ip.to_string(), prefix_len)
    } else {
        (addr.to_string(), None)
    }
}

/// Get system network configuration (DNS, gateway, hostname).
pub fn get_network_config() -> Result<NetworkConfig, AppError> {
    debug!(target: "yanos::network", "Reading system network configuration");

    let dns_servers = parse_resolv_conf_nameservers();
    let dns_search = parse_resolv_conf_search();
    let gateway = read_default_gateway();
    let hostname = read_hostname();

    debug!(
        target: "yanos::network",
        hostname = %hostname,
        dns_count = dns_servers.len(),
        gateway = ?gateway,
        "System network config read"
    );

    Ok(NetworkConfig {
        dns_servers,
        dns_search,
        gateway,
        hostname,
    })
}

/// Parse nameserver entries from /etc/resolv.conf.
fn parse_resolv_conf_nameservers() -> Vec<String> {
    let content = fs::read_to_string("/etc/resolv.conf").unwrap_or_default();
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("nameserver") {
                line.split_whitespace().nth(1).map(String::from)
            } else {
                None
            }
        })
        .collect()
}

/// Parse search domain entries from /etc/resolv.conf.
fn parse_resolv_conf_search() -> Vec<String> {
    let content = fs::read_to_string("/etc/resolv.conf").unwrap_or_default();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("search") || line.starts_with("domain") {
            return line
                .split_whitespace()
                .skip(1)
                .map(String::from)
                .collect();
        }
    }
    Vec::new()
}

/// Read default gateway from /etc/defaultrouter.
fn read_default_gateway() -> Option<String> {
    fs::read_to_string("/etc/defaultrouter")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                .map(|s| s.trim().to_string())
        })
}

/// Read hostname from /etc/nodename.
fn read_hostname() -> String {
    fs::read_to_string("/etc/nodename")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Set hostname in /etc/nodename and apply via hostname(1).
pub fn set_hostname(hostname: &str) -> Result<(), AppError> {
    debug!(target: "yanos::network", hostname, "Setting hostname");

    // Validate hostname (basic check)
    if hostname.is_empty() {
        return Err(AppError::BadRequest("Hostname cannot be empty".to_string()));
    }
    if hostname.len() > 253 {
        return Err(AppError::BadRequest("Hostname too long (max 253 chars)".to_string()));
    }
    if !hostname.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.') {
        return Err(AppError::BadRequest(
            "Hostname can only contain alphanumeric characters, hyphens, and dots".to_string()
        ));
    }

    // Write to /etc/nodename
    fs::write("/etc/nodename", format!("{}\n", hostname))
        .map_err(|e| AppError::InternalServerError(format!("Failed to write hostname: {e}")))?;

    // Apply immediately via hostname command
    let output = Command::new("hostname")
        .arg(hostname)
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to run hostname: {e}")))?;

    if !output.status.success() {
        warn!(
            target: "yanos::network",
            "hostname command failed (will apply on reboot)"
        );
    }

    debug!(target: "yanos::network", hostname, "Hostname configured");
    Ok(())
}

// =============================================================================
// Write Operations
// =============================================================================

/// Set a static IP address on an interface.
///
/// This will:
/// 1. Delete any existing address object with the same name
/// 2. Create a new static address
pub fn set_static_address(interface: &str, address: &str, prefix_len: u8) -> Result<(), AppError> {
    let addrobj = format!("{}/v4static", interface);
    let addr_with_prefix = format!("{}/{}", address, prefix_len);

    debug!(
        target: "yanos::network",
        interface,
        address = %addr_with_prefix,
        "Setting static IP address"
    );

    // First, ensure the interface is plumbed
    let _ = Command::new("ipadm")
        .args(["create-ip", interface])
        .output();

    // Delete existing address object if it exists
    let _ = Command::new("ipadm")
        .args(["delete-addr", &addrobj])
        .output();

    // Create new static address
    let output = Command::new("ipadm")
        .args(["create-addr", "-T", "static", "-a", &addr_with_prefix, &addrobj])
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to run ipadm: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::InternalServerError(format!(
            "Failed to set address: {stderr}"
        )));
    }

    Ok(())
}

/// Delete an address object.
pub fn delete_address(addrobj: &str) -> Result<(), AppError> {
    debug!(target: "yanos::network", addrobj, "Deleting address object");

    let output = Command::new("ipadm")
        .args(["delete-addr", addrobj])
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to run ipadm: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::InternalServerError(format!(
            "Failed to delete address: {stderr}"
        )));
    }

    Ok(())
}

/// Configure DHCP on an interface.
pub fn set_dhcp(interface: &str) -> Result<(), AppError> {
    let addrobj = format!("{}/dhcp", interface);

    debug!(target: "yanos::network", interface, "Configuring DHCP");

    // Ensure interface is plumbed
    let _ = Command::new("ipadm")
        .args(["create-ip", interface])
        .output();

    // Delete any existing dhcp address
    let _ = Command::new("ipadm")
        .args(["delete-addr", &addrobj])
        .output();

    // Create DHCP address
    let output = Command::new("ipadm")
        .args(["create-addr", "-T", "dhcp", &addrobj])
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to run ipadm: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::InternalServerError(format!(
            "Failed to configure DHCP: {stderr}"
        )));
    }

    Ok(())
}

/// Write DNS configuration to /etc/resolv.conf.
pub fn set_dns_config(servers: &[String], search: &[String]) -> Result<(), AppError> {
    debug!(
        target: "yanos::network",
        ?servers,
        ?search,
        "Writing DNS configuration"
    );

    let mut content = String::new();

    // Add search domains
    if !search.is_empty() {
        content.push_str(&format!("search {}\n", search.join(" ")));
    }

    // Add nameservers
    for server in servers {
        content.push_str(&format!("nameserver {}\n", server));
    }

    // Write atomically via temp file
    let temp_path = "/etc/resolv.conf.tmp";
    let mut file = fs::File::create(temp_path)
        .map_err(|e| AppError::InternalServerError(format!("Failed to create temp file: {e}")))?;

    file.write_all(content.as_bytes())
        .map_err(|e| AppError::InternalServerError(format!("Failed to write DNS config: {e}")))?;

    fs::rename(temp_path, "/etc/resolv.conf")
        .map_err(|e| AppError::InternalServerError(format!("Failed to update resolv.conf: {e}")))?;

    Ok(())
}

/// Set MTU on a physical link via dladm.
///
/// Uses `dladm set-linkprop -p mtu=<value> <link>` to configure MTU.
/// Valid MTU range is typically 576-9000 (jumbo frames).
pub fn set_mtu(link: &str, mtu: u32) -> Result<(), AppError> {
    debug!(target: "yanos::network", link, mtu, "Setting MTU");

    // Validate MTU range
    if mtu < 576 || mtu > 9000 {
        return Err(AppError::BadRequest(format!(
            "MTU must be between 576 and 9000, got {}",
            mtu
        )));
    }

    let output = Command::new("dladm")
        .args(["set-linkprop", "-p", &format!("mtu={}", mtu), link])
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to run dladm: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::InternalServerError(format!(
            "Failed to set MTU on {}: {}",
            link, stderr
        )));
    }

    debug!(target: "yanos::network", link, mtu, "MTU configured successfully");
    Ok(())
}

/// Write default gateway to /etc/defaultrouter.
pub fn set_default_gateway(gateway: &str) -> Result<(), AppError> {
    debug!(target: "yanos::network", gateway, "Setting default gateway");

    // Write to /etc/defaultrouter
    fs::write("/etc/defaultrouter", format!("{}\n", gateway))
        .map_err(|e| AppError::InternalServerError(format!("Failed to write gateway: {e}")))?;

    // Apply immediately via route command
    // First delete existing default route
    let _ = Command::new("route")
        .args(["delete", "default"])
        .output();

    // Add new default route
    let output = Command::new("route")
        .args(["add", "default", gateway])
        .output()
        .map_err(|e| AppError::InternalServerError(format!("Failed to run route: {e}")))?;

    if !output.status.success() {
        warn!(
            target: "yanos::network",
            "Failed to apply gateway immediately (will apply on reboot)"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address_prefix() {
        let (ip, prefix) = parse_address_prefix("192.168.1.10/24");
        assert_eq!(ip, "192.168.1.10");
        assert_eq!(prefix, Some(24));

        let (ip, prefix) = parse_address_prefix("10.0.0.1");
        assert_eq!(ip, "10.0.0.1");
        assert_eq!(prefix, None);

        let (ip, prefix) = parse_address_prefix("fe80::1/64");
        assert_eq!(ip, "fe80::1");
        assert_eq!(prefix, Some(64));
    }

    #[test]
    fn test_parse_resolv_conf_nameservers() {
        // This will read actual /etc/resolv.conf on the test system
        let servers = parse_resolv_conf_nameservers();
        // Just verify it returns a valid Vec (may be empty)
        let _ = servers;
    }

    #[test]
    fn test_get_network_config() {
        let config = get_network_config();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(!config.hostname.is_empty());
    }

    #[test]
    fn test_get_physical_links() {
        let links = get_physical_links();
        // Should succeed on illumos, but may return error if no physical links
        match links {
            Ok(l) => {
                println!("Found {} physical links", l.len());
                for link in &l {
                    println!("  {} - {} Mbps", link.name, link.speed);
                }
            }
            Err(e) => {
                // May fail on some VMs without physical NICs
                println!("get_physical_links returned error (may be expected): {:?}", e);
            }
        }
    }

    #[test]
    fn test_get_network_addresses() {
        let addrs = get_network_addresses();
        assert!(addrs.is_ok());
    }

    #[test]
    fn test_get_network_interfaces() {
        let ifaces = get_network_interfaces();
        assert!(ifaces.is_ok());
        let ifaces = ifaces.unwrap();
        // Should have at least one interface on any system
        println!("Found {} interfaces", ifaces.len());
        for iface in &ifaces {
            println!(
                "  {} - {} - {} - {:?} Mbps",
                iface.name, iface.state, iface.address, iface.speed
            );
        }
    }
}
