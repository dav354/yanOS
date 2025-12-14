//! Tests for TLS certificate generation and management.
//!
//! These tests verify certificate generation, loading, and reload functionality.
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Once;
use tempfile::tempdir;

use yanos_backend::tls::{ensure_tls_certs_exist, TlsState};

static CRYPTO_INIT: Once = Once::new();

fn init_crypto() {
    CRYPTO_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Test that ensure_tls_certs_exist creates certificates when missing.
#[test]
fn test_ensure_tls_certs_exist_creates_certs() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    // Certificates should not exist yet
    assert!(!cert_dir.join("cert.pem").exists());
    assert!(!cert_dir.join("key.pem").exists());

    // Generate certificates
    let result = ensure_tls_certs_exist(cert_dir);
    assert!(result.is_ok(), "Should generate certificates: {:?}", result);

    // Certificates should now exist
    assert!(cert_dir.join("cert.pem").exists(), "cert.pem should exist");
    assert!(cert_dir.join("key.pem").exists(), "key.pem should exist");
}

/// Test that ensure_tls_certs_exist doesn't regenerate existing certs.
#[test]
fn test_ensure_tls_certs_exist_preserves_existing() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    // Generate initial certificates
    ensure_tls_certs_exist(cert_dir).expect("Should generate certificates");

    // Read original content
    let original_cert = fs::read_to_string(cert_dir.join("cert.pem")).unwrap();
    let original_key = fs::read_to_string(cert_dir.join("key.pem")).unwrap();

    // Call again
    ensure_tls_certs_exist(cert_dir).expect("Should succeed with existing certs");

    // Content should be unchanged
    let new_cert = fs::read_to_string(cert_dir.join("cert.pem")).unwrap();
    let new_key = fs::read_to_string(cert_dir.join("key.pem")).unwrap();

    assert_eq!(original_cert, new_cert, "Certificate should not change");
    assert_eq!(original_key, new_key, "Key should not change");
}

/// Test that generated certificates have correct permissions.
#[test]
fn test_certificate_permissions() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    ensure_tls_certs_exist(cert_dir).expect("Should generate certificates");

    // Check cert permissions (should be 0o600)
    let cert_perms = fs::metadata(cert_dir.join("cert.pem"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(
        cert_perms & 0o777,
        0o600,
        "cert.pem should have 600 permissions"
    );

    // Check key permissions (should be 0o600)
    let key_perms = fs::metadata(cert_dir.join("key.pem"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(
        key_perms & 0o777,
        0o600,
        "key.pem should have 600 permissions"
    );
}

/// Test that generated certificate is valid PEM.
#[test]
fn test_certificate_is_valid_pem() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    ensure_tls_certs_exist(cert_dir).expect("Should generate certificates");

    let cert_content = fs::read_to_string(cert_dir.join("cert.pem")).unwrap();
    let key_content = fs::read_to_string(cert_dir.join("key.pem")).unwrap();

    // Check PEM headers
    assert!(
        cert_content.contains("-----BEGIN CERTIFICATE-----"),
        "Certificate should have PEM header"
    );
    assert!(
        cert_content.contains("-----END CERTIFICATE-----"),
        "Certificate should have PEM footer"
    );

    assert!(
        key_content.contains("-----BEGIN"),
        "Key should have PEM header"
    );
    assert!(key_content.contains("-----END"), "Key should have PEM footer");
}

/// Test TlsState::load with valid certificates.
#[tokio::test]
async fn test_tls_state_load() {
    init_crypto();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    // Generate certificates first
    ensure_tls_certs_exist(cert_dir).expect("Should generate certificates");

    // Load TLS state
    let result = TlsState::load(cert_dir).await;
    assert!(result.is_ok(), "Should load TLS state: {:?}", result);

    let state = result.unwrap();
    assert!(state.is_ready(), "TLS state should be ready");
}

/// Test TlsState::load creates certs if missing.
#[tokio::test]
async fn test_tls_state_load_generates_certs() {
    init_crypto();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    // Don't pre-generate - TlsState::load should do it
    let result = TlsState::load(cert_dir).await;
    assert!(result.is_ok(), "Should load TLS state: {:?}", result);

    // Certificates should now exist
    assert!(cert_dir.join("cert.pem").exists());
    assert!(cert_dir.join("key.pem").exists());
}

/// Test TlsState::is_ready after successful load.
#[tokio::test]
async fn test_tls_state_is_ready() {
    init_crypto();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    let state = TlsState::load(cert_dir)
        .await
        .expect("Should load TLS state");

    assert!(state.is_ready(), "Should be ready after successful load");
}

/// Test TlsState::config returns usable config.
#[tokio::test]
async fn test_tls_state_config() {
    init_crypto();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    let state = TlsState::load(cert_dir)
        .await
        .expect("Should load TLS state");

    let config = state.config();
    // Config should be clonable
    let _config2 = config.clone();
}

/// Test ensure_tls_certs_exist with nested directory creation.
#[test]
fn test_ensure_tls_certs_exist_creates_nested_dirs() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let nested_dir = temp_dir.path().join("deeply").join("nested").join("tls");

    // Nested directory should not exist
    assert!(!nested_dir.exists());

    // Should create nested directories and certificates
    let result = ensure_tls_certs_exist(&nested_dir);
    assert!(
        result.is_ok(),
        "Should create nested directories: {:?}",
        result
    );

    assert!(nested_dir.join("cert.pem").exists());
    assert!(nested_dir.join("key.pem").exists());
}

/// Test DEFAULT_TLS_DIR constant.
#[test]
fn test_default_tls_dir() {
    use yanos_backend::tls::DEFAULT_TLS_DIR;

    assert_eq!(DEFAULT_TLS_DIR, "/etc/opt/yanos/tls");
}

/// Test loading invalid certificate fails gracefully.
#[tokio::test]
async fn test_tls_state_load_invalid_cert() {
    init_crypto();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    // Create invalid certificate files
    fs::write(cert_dir.join("cert.pem"), "not a valid certificate").unwrap();
    fs::write(cert_dir.join("key.pem"), "not a valid key").unwrap();

    // Should fail to load
    let result = TlsState::load(cert_dir).await;
    assert!(result.is_err(), "Should fail with invalid certificates");
}

/// Test loading with only cert (missing key) fails.
#[tokio::test]
async fn test_tls_state_load_missing_key() {
    init_crypto();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    // Generate valid cert
    ensure_tls_certs_exist(cert_dir).expect("Should generate certificates");

    // Remove key
    fs::remove_file(cert_dir.join("key.pem")).unwrap();

    // TlsState::load should regenerate
    let result = TlsState::load(cert_dir).await;
    assert!(result.is_ok(), "Should regenerate missing files");
}

/// Test loading with only key (missing cert) fails.
#[tokio::test]
async fn test_tls_state_load_missing_cert() {
    init_crypto();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let cert_dir = temp_dir.path();

    // Generate valid cert
    ensure_tls_certs_exist(cert_dir).expect("Should generate certificates");

    // Remove cert
    fs::remove_file(cert_dir.join("cert.pem")).unwrap();

    // TlsState::load should regenerate
    let result = TlsState::load(cert_dir).await;
    assert!(result.is_ok(), "Should regenerate missing files");
}
