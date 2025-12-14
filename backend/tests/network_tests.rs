//! Tests for the network adapter module.
//!
//! These tests verify the network interface discovery functions that
//! interface with illumos ipadm(8).
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use yanos_backend::adapters::get_network_interfaces;
use yanos_backend::core::NetworkInterface;
use yanos_backend::error::AppError;

/// Test that get_network_interfaces returns a valid list.
#[test]
fn test_get_network_interfaces() {
    let result = get_network_interfaces();

    match result {
        Ok(interfaces) => {
            println!("Found {} network interfaces", interfaces.len());

            // Verify structure of returned interfaces
            for iface in &interfaces {
                assert!(!iface.name.is_empty(), "Interface name should not be empty");
                assert!(!iface.state.is_empty(), "Interface state should not be empty");
                // Address could be empty in some states, but typically isn't
                println!(
                    "  {} - state: {}, addr: {}",
                    iface.name, iface.state, iface.address
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
        Err(AppError::ServiceUnavailable(msg)) => {
            // Expected if ipadm is unavailable
            println!("Network interfaces unavailable: {}", msg);
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

/// Test NetworkInterface structure.
#[test]
fn test_network_interface_structure() {
    let iface = NetworkInterface {
        name: "e1000g0/v4".to_string(),
        state: "ok".to_string(),
        address: "192.168.1.100/24".to_string(),
    };

    assert_eq!(iface.name, "e1000g0/v4");
    assert_eq!(iface.state, "ok");
    assert_eq!(iface.address, "192.168.1.100/24");
}

/// Test NetworkInterface serialization.
#[test]
fn test_network_interface_serialization() {
    let iface = NetworkInterface {
        name: "vnic0/v4".to_string(),
        state: "ok".to_string(),
        address: "10.0.0.1/8".to_string(),
    };

    let json = serde_json::to_string(&iface).expect("Serialization failed");
    assert!(json.contains("\"name\":\"vnic0/v4\""));
    assert!(json.contains("\"state\":\"ok\""));
    assert!(json.contains("\"address\":\"10.0.0.1/8\""));
}

/// Test NetworkInterface Clone and Debug.
#[test]
fn test_network_interface_clone_debug() {
    let iface = NetworkInterface {
        name: "ixgbe0/v6".to_string(),
        state: "tentative".to_string(),
        address: "fe80::1/64".to_string(),
    };

    let cloned = iface.clone();
    assert_eq!(cloned.name, iface.name);

    let debug_str = format!("{:?}", iface);
    assert!(debug_str.contains("NetworkInterface"));
    assert!(debug_str.contains("ixgbe0/v6"));
}

/// Test that interfaces with various states are handled.
#[test]
fn test_interface_state_variations() {
    // These are common ipadm states on illumos
    let states = ["ok", "tentative", "down", "disabled", "duplicated"];

    for state in states {
        let iface = NetworkInterface {
            name: "test0/v4".to_string(),
            state: state.to_string(),
            address: "0.0.0.0".to_string(),
        };

        // Should serialize without error
        let json = serde_json::to_string(&iface).expect("Serialization failed");
        assert!(json.contains(&format!("\"state\":\"{}\"", state)));
    }
}
