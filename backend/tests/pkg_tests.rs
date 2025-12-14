//! Tests for the pkg adapter module.
//!
//! These tests verify the package management functions that interface
//! with the illumos pkg(7) system. Tests are designed to work on both
//! Linux (where pkg is unavailable) and illumos.

use yanos_backend::adapters::pkg;
use yanos_backend::core::PackageInfo;
use yanos_backend::error::AppError;

/// Test the parse_fmri function indirectly through get_pkg_list.
/// On non-illumos systems, this tests the error handling path.
#[test]
fn test_get_pkg_list() {
    let result = pkg::get_pkg_list();

    match result {
        Ok(packages) => {
            // On illumos with packages installed
            println!("Found {} packages", packages.len());

            // Should have at least some core system packages
            assert!(!packages.is_empty(), "Package list should not be empty on illumos");

            // Verify package structure
            for pkg in packages.iter().take(5) {
                assert!(!pkg.name.is_empty(), "Package name should not be empty");
                // Version could be empty for some edge cases, but status should exist
                assert!(!pkg.status.is_empty(), "Package status should not be empty");
                println!("  {} v{} ({})", pkg.name, pkg.version, pkg.status);
            }
        }
        Err(AppError::ServiceUnavailable(msg)) => {
            // Expected on non-illumos systems
            println!("pkg list unavailable (expected on non-illumos): {}", msg);
        }
        Err(e) => {
            // Unexpected error type
            println!("pkg list returned error: {:?}", e);
        }
    }
}

/// Test the get_pkg_updates function.
#[test]
fn test_get_pkg_updates() {
    let result = pkg::get_pkg_updates();

    match result {
        Ok(updates) => {
            // Could be empty if system is up to date
            println!("Found {} package updates available", updates.len());

            for upd in &updates {
                assert!(!upd.name.is_empty(), "Update package name should not be empty");
                // Updates should have the NEW version info
                println!("  {} -> v{} ({})", upd.name, upd.version, upd.status);
            }
        }
        Err(AppError::ServiceUnavailable(msg)) => {
            // Expected on non-illumos systems
            println!("pkg updates unavailable (expected on non-illumos): {}", msg);
        }
        Err(e) => {
            println!("pkg updates returned error: {:?}", e);
        }
    }
}

/// Test that refresh_catalog handles errors gracefully.
#[test]
fn test_refresh_catalog() {
    let result = pkg::refresh_catalog();

    match result {
        Ok(()) => {
            println!("pkg refresh succeeded");
        }
        Err(AppError::InternalServerError(msg)) => {
            // Expected if pkg command fails
            println!("pkg refresh failed (expected on non-illumos): {}", msg);
        }
        Err(e) => {
            println!("pkg refresh returned error: {:?}", e);
        }
    }
}

/// Test PackageInfo structure.
#[test]
fn test_package_info_structure() {
    let pkg = PackageInfo {
        name: "system/kernel".to_string(),
        version: "11.4.0.15.0".to_string(),
        build_time: "20231215T120000Z".to_string(),
        status: "i--".to_string(),
    };

    assert_eq!(pkg.name, "system/kernel");
    assert_eq!(pkg.version, "11.4.0.15.0");
    assert_eq!(pkg.build_time, "20231215T120000Z");
    assert_eq!(pkg.status, "i--");
}

/// Test PackageInfo serialization.
#[test]
fn test_package_info_serialization() {
    let pkg = PackageInfo {
        name: "developer/rust".to_string(),
        version: "1.74.0".to_string(),
        build_time: "20231201T000000Z".to_string(),
        status: "i--".to_string(),
    };

    let json = serde_json::to_string(&pkg).expect("Serialization failed");
    assert!(json.contains("\"name\":\"developer/rust\""));
    assert!(json.contains("\"version\":\"1.74.0\""));
    assert!(json.contains("\"build_time\":\"20231201T000000Z\""));
    assert!(json.contains("\"status\":\"i--\""));

    let deserialized: PackageInfo = serde_json::from_str(&json)
        .expect("Deserialization failed");
    assert_eq!(deserialized.name, pkg.name);
    assert_eq!(deserialized.version, pkg.version);
}

/// Test that update packages have upgrade_available status.
#[test]
fn test_update_status_format() {
    let result = pkg::get_pkg_updates();

    if let Ok(updates) = result {
        for upd in &updates {
            assert_eq!(
                upd.status, "upgrade_available",
                "Update packages should have 'upgrade_available' status"
            );
        }
    }
}

// --- Edge Case Tests ---

/// Test PackageInfo with empty strings.
#[test]
fn test_package_info_empty_strings() {
    let pkg = PackageInfo {
        name: "".to_string(),
        version: "".to_string(),
        build_time: "".to_string(),
        status: "".to_string(),
    };

    // Should serialize without panic
    let json = serde_json::to_string(&pkg).expect("Serialization failed");
    assert!(json.contains("\"name\":\"\""));
}

/// Test PackageInfo with very long values.
#[test]
fn test_package_info_long_values() {
    let long_name = "package/".to_string() + &"x".repeat(1000);
    let pkg = PackageInfo {
        name: long_name.clone(),
        version: "1.0.0".to_string(),
        build_time: "20231215T120000Z".to_string(),
        status: "i--".to_string(),
    };

    let json = serde_json::to_string(&pkg).expect("Serialization failed");
    let deserialized: PackageInfo = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.name, long_name);
}

/// Test PackageInfo with special characters.
#[test]
fn test_package_info_special_chars() {
    let pkg = PackageInfo {
        name: "package/with-special_chars.2024".to_string(),
        version: "1.2.3-rc1+build.456".to_string(),
        build_time: "2024-01-15T12:30:45Z".to_string(),
        status: "i--".to_string(),
    };

    let json = serde_json::to_string(&pkg).expect("Serialization failed");
    let deserialized: PackageInfo = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.name, pkg.name);
    assert_eq!(deserialized.version, pkg.version);
}

/// Test PackageInfo Clone and Debug.
#[test]
fn test_package_info_clone_debug() {
    let pkg = PackageInfo {
        name: "test/pkg".to_string(),
        version: "1.0".to_string(),
        build_time: "now".to_string(),
        status: "i--".to_string(),
    };

    let cloned = pkg.clone();
    assert_eq!(cloned.name, pkg.name);

    let debug_str = format!("{:?}", pkg);
    assert!(debug_str.contains("PackageInfo"));
    assert!(debug_str.contains("test/pkg"));
}

/// Test various package status values.
#[test]
fn test_package_status_values() {
    // Common pkg status flags from `pkg list`
    let statuses = [
        "i--",       // installed
        "if-",       // installed, frozen
        "ifr",       // installed, frozen, renamed
        "---",       // not installed
        "upgrade_available",
    ];

    for status in statuses {
        let pkg = PackageInfo {
            name: "test".to_string(),
            version: "1.0".to_string(),
            build_time: "".to_string(),
            status: status.to_string(),
        };

        let json = serde_json::to_string(&pkg).expect("Serialization failed");
        assert!(json.contains(&format!("\"status\":\"{}\"", status)));
    }
}

/// Test package name with publisher prefix.
#[test]
fn test_package_name_with_publisher() {
    let pkg = PackageInfo {
        name: "omnios.ms/system/kernel".to_string(),
        version: "11.4".to_string(),
        build_time: "20240115T000000Z".to_string(),
        status: "i--".to_string(),
    };

    assert!(pkg.name.contains("/"));
    let json = serde_json::to_string(&pkg).expect("Serialization failed");
    assert!(json.contains("omnios.ms/system/kernel"));
}

/// Test refresh_catalog error handling.
#[test]
fn test_refresh_catalog_error_handling() {
    // This test verifies the function handles errors gracefully
    // On a system without pkg, it should return an error without panicking
    let result = pkg::refresh_catalog();
    // Either succeeds or returns appropriate error
    match result {
        Ok(()) => println!("Refresh succeeded"),
        Err(e) => println!("Refresh error (expected on non-illumos): {:?}", e),
    }
}
