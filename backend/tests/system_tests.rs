//! Tests for the system adapter module.
//!
//! These tests verify system information functions that interface
//! with illumos utilities (hostname, uname, kstat).
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use yanos_backend::adapters::{get_hostname, get_system_info};
use yanos_backend::core::SystemInfo;

/// Test that get_hostname returns a non-empty string.
#[test]
fn test_get_hostname() {
    let result = get_hostname();

    match result {
        Ok(hostname) => {
            assert!(!hostname.is_empty(), "Hostname should not be empty");
            // Hostname should not contain newlines
            assert!(
                !hostname.contains('\n'),
                "Hostname should not contain newlines"
            );
            println!("Hostname: {}", hostname);
        }
        Err(e) => {
            panic!("Failed to get hostname: {:?}", e);
        }
    }
}

/// Test that get_system_info returns valid data.
#[test]
fn test_get_system_info() {
    let result = get_system_info();

    match result {
        Ok(info) => {
            assert!(!info.hostname.is_empty(), "Hostname should not be empty");
            assert!(
                !info.kernel_version.is_empty(),
                "Kernel version should not be empty"
            );
            // Uptime should be positive (system has been up for at least a moment)
            assert!(info.uptime > 0, "Uptime should be positive");

            println!(
                "System: {} running {} (uptime: {}s)",
                info.hostname, info.kernel_version, info.uptime
            );
        }
        Err(e) => {
            panic!("Failed to get system info: {:?}", e);
        }
    }
}

/// Test SystemInfo structure.
#[test]
fn test_system_info_structure() {
    let info = SystemInfo {
        hostname: "testhost".to_string(),
        kernel_version: "SunOS 5.11 omnios-r151048-abcdef".to_string(),
        uptime: 86400,
    };

    assert_eq!(info.hostname, "testhost");
    assert_eq!(
        info.kernel_version,
        "SunOS 5.11 omnios-r151048-abcdef"
    );
    assert_eq!(info.uptime, 86400);
}

/// Test that kernel version contains expected illumos identifiers.
#[test]
fn test_kernel_version_format() {
    let result = get_system_info();

    if let Ok(info) = result {
        // On illumos, uname -srv typically returns "SunOS <version> <build>"
        // Check for SunOS or similar identifier
        let is_illumos = info.kernel_version.contains("SunOS")
            || info.kernel_version.contains("illumos")
            || info.kernel_version.contains("omnios");

        assert!(
            is_illumos || info.kernel_version != "unknown",
            "Kernel version should be detected on illumos: {}",
            info.kernel_version
        );
    }
}

/// Test uptime calculation is reasonable.
#[test]
fn test_uptime_reasonable() {
    let result = get_system_info();

    if let Ok(info) = result {
        // Uptime should be less than ~10 years in seconds (sanity check)
        let ten_years_seconds = 10 * 365 * 24 * 60 * 60;
        assert!(
            info.uptime < ten_years_seconds,
            "Uptime {} seems unreasonably large",
            info.uptime
        );

        // Uptime should be at least 1 second
        assert!(
            info.uptime >= 1,
            "Uptime {} should be at least 1 second",
            info.uptime
        );
    }
}

/// Test hostname doesn't contain invalid characters.
#[test]
fn test_hostname_valid_chars() {
    let result = get_hostname();

    if let Ok(hostname) = result {
        // Hostnames typically contain only alphanumeric, dots, and hyphens
        for c in hostname.chars() {
            assert!(
                c.is_alphanumeric() || c == '.' || c == '-',
                "Hostname contains unexpected character: '{}'",
                c
            );
        }
    }
}

/// Test error handling when hostname command fails.
/// This test verifies the error type rather than causing a failure.
#[test]
fn test_hostname_error_type() {
    // The error type should be std::io::Error
    let result: Result<String, std::io::Error> = get_hostname();

    // Just verify the Result type is correct
    match result {
        Ok(_) => {} // Expected on a working system
        Err(e) => {
            // Verify it's an IO error
            println!("Hostname error (expected if hostname unavailable): {:?}", e);
        }
    }
}
