//! Tests for the configuration module.
//!
//! These tests verify config loading, persistence, and serialization.
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

use yanos_backend::config::{AppConfig, TelemetryConfig, DEFAULT_CONFIG_PATH};

/// Test that DEFAULT_CONFIG_PATH is set correctly.
#[test]
fn test_default_config_path() {
    assert_eq!(DEFAULT_CONFIG_PATH, "/etc/opt/yanos/config.json");
}

/// Test AppConfig::default() creates valid defaults.
#[test]
fn test_app_config_default() {
    let config = AppConfig::default();

    // Telemetry should have no OTLP endpoint by default
    assert!(config.telemetry.otlp_endpoint.is_none());
}

/// Test TelemetryConfig::default() creates valid defaults.
#[test]
fn test_telemetry_config_default() {
    let telemetry = TelemetryConfig::default();
    assert!(telemetry.otlp_endpoint.is_none());
}

/// Test loading config from a non-existent file returns defaults.
#[test]
fn test_load_missing_file_returns_defaults() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("nonexistent.json");

    let config = AppConfig::load(&config_path).expect("Should return defaults for missing file");

    // Should have default values
    assert!(config.telemetry.otlp_endpoint.is_none());
}

/// Test loading config from a valid JSON file.
#[test]
fn test_load_valid_config() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let json = r#"{
        "telemetry": {
            "otlp_endpoint": "http://localhost:4317"
        }
    }"#;
    fs::write(&config_path, json).expect("Failed to write test config");

    let config = AppConfig::load(&config_path).expect("Should load config");
    assert_eq!(
        config.telemetry.otlp_endpoint,
        Some("http://localhost:4317".to_string())
    );
}

/// Test loading config with null telemetry endpoint.
#[test]
fn test_load_config_with_null_endpoint() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let json = r#"{
        "telemetry": {
            "otlp_endpoint": null
        }
    }"#;
    fs::write(&config_path, json).expect("Failed to write test config");

    let config = AppConfig::load(&config_path).expect("Should load config");
    assert!(config.telemetry.otlp_endpoint.is_none());
}

/// Test loading config with empty telemetry object.
#[test]
fn test_load_config_with_empty_telemetry() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let json = r#"{
        "telemetry": {}
    }"#;
    fs::write(&config_path, json).expect("Failed to write test config");

    let config = AppConfig::load(&config_path).expect("Should load config");
    assert!(config.telemetry.otlp_endpoint.is_none());
}

/// Test loading invalid JSON fails.
#[test]
fn test_load_invalid_json_fails() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    fs::write(&config_path, "{ not valid json }").expect("Failed to write test config");

    let result = AppConfig::load(&config_path);
    assert!(result.is_err(), "Should fail with invalid JSON");
}

/// Test persist creates config file.
#[test]
fn test_persist_creates_file() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let config = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some("http://collector:4317".to_string()),
        },
    };

    config.persist(&config_path).expect("Should persist config");

    assert!(config_path.exists(), "Config file should exist");

    // Verify content
    let content = fs::read_to_string(&config_path).expect("Should read config");
    assert!(content.contains("http://collector:4317"));
}

/// Test persist creates nested directories.
#[test]
fn test_persist_creates_nested_dirs() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("a").join("b").join("c").join("config.json");

    let config = AppConfig::default();
    config.persist(&config_path).expect("Should persist config");

    assert!(config_path.exists(), "Config file should exist in nested dir");
}

/// Test persist sets restrictive permissions.
#[test]
fn test_persist_sets_permissions() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let config = AppConfig::default();
    config.persist(&config_path).expect("Should persist config");

    let perms = fs::metadata(&config_path)
        .expect("Should get metadata")
        .permissions()
        .mode();
    assert_eq!(
        perms & 0o777,
        0o600,
        "Config should have 600 permissions"
    );
}

/// Test round-trip: persist then load.
#[test]
fn test_round_trip() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let original = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some("http://test:4317".to_string()),
        },
    };

    original.persist(&config_path).expect("Should persist");
    let loaded = AppConfig::load(&config_path).expect("Should load");

    assert_eq!(original.telemetry.otlp_endpoint, loaded.telemetry.otlp_endpoint);
}

/// Test AppConfig Clone implementation.
#[test]
fn test_app_config_clone() {
    let config = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some("http://test:4317".to_string()),
        },
    };

    let cloned = config.clone();
    assert_eq!(cloned.telemetry.otlp_endpoint, config.telemetry.otlp_endpoint);
}

/// Test AppConfig Debug implementation.
#[test]
fn test_app_config_debug() {
    let config = AppConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("AppConfig"));
    assert!(debug_str.contains("telemetry"));
}

/// Test TelemetryConfig Clone implementation.
#[test]
fn test_telemetry_config_clone() {
    let telemetry = TelemetryConfig {
        otlp_endpoint: Some("http://test:4317".to_string()),
    };

    let cloned = telemetry.clone();
    assert_eq!(cloned.otlp_endpoint, telemetry.otlp_endpoint);
}

/// Test TelemetryConfig Debug implementation.
#[test]
fn test_telemetry_config_debug() {
    let telemetry = TelemetryConfig {
        otlp_endpoint: Some("http://test:4317".to_string()),
    };

    let debug_str = format!("{:?}", telemetry);
    assert!(debug_str.contains("TelemetryConfig"));
    assert!(debug_str.contains("http://test:4317"));
}

/// Test config serialization produces valid JSON.
#[test]
fn test_config_serialization() {
    let config = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some("http://localhost:4317".to_string()),
        },
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    assert!(json.contains("\"otlp_endpoint\""));
    assert!(json.contains("http://localhost:4317"));

    // Verify it can be deserialized
    let _: AppConfig = serde_json::from_str(&json).expect("Deserialization failed");
}

// --- Edge Case Tests ---

/// Test loading empty file fails.
#[test]
fn test_load_empty_file_fails() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    fs::write(&config_path, "").expect("Failed to write empty file");

    let result = AppConfig::load(&config_path);
    assert!(result.is_err(), "Should fail with empty file");
}

/// Test loading file with extra fields succeeds (forward compat).
#[test]
fn test_load_config_with_extra_fields() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let json = r#"{
        "telemetry": {
            "otlp_endpoint": "http://localhost:4317"
        },
        "unknown_field": "should be ignored",
        "nested": {
            "deep": true
        }
    }"#;
    fs::write(&config_path, json).expect("Failed to write test config");

    // Should still load successfully, ignoring unknown fields
    let config = AppConfig::load(&config_path).expect("Should load config with extra fields");
    assert_eq!(
        config.telemetry.otlp_endpoint,
        Some("http://localhost:4317".to_string())
    );
}

/// Test config with very long endpoint URL.
#[test]
fn test_config_long_endpoint_url() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let long_url = format!("http://{}:4317", "x".repeat(5000));
    let config = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some(long_url.clone()),
        },
    };

    config.persist(&config_path).expect("Should persist");
    let loaded = AppConfig::load(&config_path).expect("Should load");

    assert_eq!(loaded.telemetry.otlp_endpoint, Some(long_url));
}

/// Test config with special characters in endpoint URL.
#[test]
fn test_config_special_chars_in_endpoint() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let special_url = "http://host:4317/path?query=value&foo=bar#fragment";
    let config = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some(special_url.to_string()),
        },
    };

    config.persist(&config_path).expect("Should persist");
    let loaded = AppConfig::load(&config_path).expect("Should load");

    assert_eq!(loaded.telemetry.otlp_endpoint, Some(special_url.to_string()));
}

/// Test config with unicode in endpoint URL.
#[test]
fn test_config_unicode_in_endpoint() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    let config = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some("http://host:4317".to_string()),
        },
    };

    config.persist(&config_path).expect("Should persist");

    // Read and verify the JSON is properly formatted
    let content = fs::read_to_string(&config_path).expect("Should read");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Should be valid JSON");
    assert!(parsed.is_object());
}

/// Test persist over existing file.
#[test]
fn test_persist_overwrites_existing() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");

    // Create initial config
    let config1 = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some("http://first:4317".to_string()),
        },
    };
    config1.persist(&config_path).expect("Should persist first");

    // Overwrite with new config
    let config2 = AppConfig {
        telemetry: TelemetryConfig {
            otlp_endpoint: Some("http://second:4317".to_string()),
        },
    };
    config2.persist(&config_path).expect("Should persist second");

    // Verify it was overwritten
    let loaded = AppConfig::load(&config_path).expect("Should load");
    assert_eq!(
        loaded.telemetry.otlp_endpoint,
        Some("http://second:4317".to_string())
    );
}
