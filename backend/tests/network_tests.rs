//! Tests for the network adapter module.
//!
//! These tests verify the network interface discovery and configuration functions
//! that interface with illumos ipadm(8) and dladm(8).

use yanos_backend::adapters::{get_network_config, get_network_interfaces, get_physical_links};
use yanos_backend::core::{NetworkConfig, NetworkInterface, PhysicalLink};

/// Test that get_network_interfaces returns a valid list.
#[test]
fn test_get_network_interfaces() {
    let result = get_network_interfaces();
    assert!(result.is_ok(), "get_network_interfaces should succeed");

    let interfaces = result.unwrap();
    println!("Found {} network interfaces", interfaces.len());

    for iface in &interfaces {
        assert!(!iface.name.is_empty(), "Interface name should not be empty");
        assert!(!iface.state.is_empty(), "Interface state should not be empty");
        println!(
            "  {} - state: {}, addr: {}, speed: {:?} Mbps",
            iface.name, iface.state, iface.address, iface.speed
        );
    }

    // Loopback should be filtered out
    for iface in &interfaces {
        assert!(
            !iface.name.starts_with("lo"),
            "Loopback interface should be filtered out: {}",
            iface.name
        );
    }
}

/// Test that get_physical_links returns valid data.
#[test]
fn test_get_physical_links() {
    let result = get_physical_links();
    // May fail on VMs without physical NICs
    match result {
        Ok(links) => {
            println!("Found {} physical links", links.len());
            for link in &links {
                assert!(!link.name.is_empty(), "Link name should not be empty");
                println!(
                    "  {} - state: {}, speed: {} Mbps, mac: {}",
                    link.name, link.state, link.speed, link.mac
                );
            }
        }
        Err(e) => {
            println!("get_physical_links returned error (may be expected on VM): {:?}", e);
        }
    }
}

/// Test that get_network_config returns valid configuration.
#[test]
fn test_get_network_config() {
    let result = get_network_config();
    assert!(result.is_ok(), "get_network_config should succeed");

    let config = result.unwrap();
    assert!(!config.hostname.is_empty(), "Hostname should not be empty");
    println!("Hostname: {}", config.hostname);
    println!("DNS servers: {:?}", config.dns_servers);
    println!("DNS search: {:?}", config.dns_search);
    println!("Gateway: {:?}", config.gateway);
}

/// Test NetworkInterface structure.
#[test]
fn test_network_interface_structure() {
    let iface = NetworkInterface {
        name: "e1000g0".to_string(),
        state: "up".to_string(),
        address: "192.168.1.100".to_string(),
        prefix_len: Some(24),
        mac: Some("00:11:22:33:44:55".to_string()),
        speed: Some(1000),
        mtu: Some(1500),
        addr_type: Some("static".to_string()),
        friendly_name: Some("Management".to_string()),
    };

    assert_eq!(iface.name, "e1000g0");
    assert_eq!(iface.state, "up");
    assert_eq!(iface.address, "192.168.1.100");
    assert_eq!(iface.prefix_len, Some(24));
    assert_eq!(iface.speed, Some(1000));
}

/// Test NetworkInterface serialization.
#[test]
fn test_network_interface_serialization() {
    let iface = NetworkInterface {
        name: "vnic0".to_string(),
        state: "up".to_string(),
        address: "10.0.0.1".to_string(),
        prefix_len: Some(8),
        mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
        speed: Some(10000),
        mtu: Some(9000),
        addr_type: Some("dhcp".to_string()),
        friendly_name: None,
    };

    let json = serde_json::to_string(&iface).expect("Serialization failed");
    assert!(json.contains("\"name\":\"vnic0\""));
    assert!(json.contains("\"state\":\"up\""));
    assert!(json.contains("\"speed\":10000"));
    // friendly_name should be skipped because it's None
    assert!(!json.contains("friendly_name"));
}

/// Test NetworkInterface Clone and Debug.
#[test]
fn test_network_interface_clone_debug() {
    let iface = NetworkInterface {
        name: "ixgbe0".to_string(),
        state: "up".to_string(),
        address: "fe80::1".to_string(),
        prefix_len: Some(64),
        mac: None,
        speed: Some(10000),
        mtu: Some(1500),
        addr_type: None,
        friendly_name: None,
    };

    let cloned = iface.clone();
    assert_eq!(cloned.name, iface.name);

    let debug_str = format!("{:?}", iface);
    assert!(debug_str.contains("NetworkInterface"));
    assert!(debug_str.contains("ixgbe0"));
}

/// Test PhysicalLink structure.
#[test]
fn test_physical_link_structure() {
    let link = PhysicalLink {
        name: "e1000g0".to_string(),
        media: "Ethernet".to_string(),
        state: "up".to_string(),
        speed: 1000,
        duplex: "full".to_string(),
        mac: "00:11:22:33:44:55".to_string(),
        mtu: 1500,
        friendly_name: Some("Primary NIC".to_string()),
    };

    assert_eq!(link.name, "e1000g0");
    assert_eq!(link.speed, 1000);
    assert_eq!(link.duplex, "full");
}

/// Test PhysicalLink serialization.
#[test]
fn test_physical_link_serialization() {
    let link = PhysicalLink {
        name: "ixgbe0".to_string(),
        media: "Ethernet".to_string(),
        state: "up".to_string(),
        speed: 10000,
        duplex: "full".to_string(),
        mac: "aa:bb:cc:dd:ee:ff".to_string(),
        mtu: 9000,
        friendly_name: None,
    };

    let json = serde_json::to_string(&link).expect("Serialization failed");
    assert!(json.contains("\"name\":\"ixgbe0\""));
    assert!(json.contains("\"speed\":10000"));
    assert!(json.contains("\"mtu\":9000"));
}

/// Test NetworkConfig structure.
#[test]
fn test_network_config_structure() {
    let config = NetworkConfig {
        dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
        dns_search: vec!["example.com".to_string()],
        gateway: Some("192.168.1.1".to_string()),
        hostname: "myhost".to_string(),
    };

    assert_eq!(config.dns_servers.len(), 2);
    assert_eq!(config.hostname, "myhost");
    assert!(config.gateway.is_some());
}

/// Test NetworkConfig serialization.
#[test]
fn test_network_config_serialization() {
    let config = NetworkConfig {
        dns_servers: vec!["1.1.1.1".to_string()],
        dns_search: vec![],
        gateway: None,
        hostname: "test".to_string(),
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    assert!(json.contains("\"hostname\":\"test\""));
    assert!(json.contains("\"dns_servers\":[\"1.1.1.1\"]"));
}

/// Test that interfaces with various states are handled.
#[test]
fn test_interface_state_variations() {
    let states = ["up", "down", "ok", "tentative", "disabled"];

    for state in states {
        let iface = NetworkInterface {
            name: "test0".to_string(),
            state: state.to_string(),
            address: "0.0.0.0".to_string(),
            prefix_len: None,
            mac: None,
            speed: None,
            mtu: None,
            addr_type: None,
            friendly_name: None,
        };

        let json = serde_json::to_string(&iface).expect("Serialization failed");
        assert!(json.contains(&format!("\"state\":\"{}\"", state)));
    }
}

/// Test interface without optional fields.
#[test]
fn test_minimal_interface() {
    let iface = NetworkInterface {
        name: "eth0".to_string(),
        state: "up".to_string(),
        address: String::new(),
        prefix_len: None,
        mac: None,
        speed: None,
        mtu: None,
        addr_type: None,
        friendly_name: None,
    };

    let json = serde_json::to_string(&iface).expect("Serialization failed");
    // Optional fields with None should be skipped
    assert!(!json.contains("prefix_len"));
    assert!(!json.contains("mac"));
    assert!(!json.contains("speed"));
}
