// backend/src/tls.rs

use rcgen::generate_simple_self_signed;
use std::fs;
use std::io;
use std::path::Path;
use tracing::info;

const CERT_PATH: &str = "/etc/opt/storage-os/tls/cert.pem";
const KEY_PATH: &str = "/etc/opt/storage-os/tls/key.pem";

/// Ensures that TLS certificate and key exist, generating them if they don't.
pub fn ensure_tls_certs_exist() -> io::Result<()> {
    let cert_path = Path::new(CERT_PATH);
    let key_path = Path::new(KEY_PATH);

    if cert_path.exists() && key_path.exists() {
        info!("TLS certificate and key found.");
        return Ok(());
    }

    info!("TLS certificate or key not found. Generating self-signed certificate...");

    // Create the directory if it doesn't exist.
    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Generate a new self-signed certificate.
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let certified_key = generate_simple_self_signed(subject_alt_names).map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to generate self-signed certificate: {err}"),
        )
    })?;
    let cert_pem = certified_key.cert.pem();
    let key_pem = certified_key.signing_key.serialize_pem();

    // Write the certificate and key to their respective files.
    fs::write(cert_path, cert_pem)?;
    fs::write(key_path, key_pem)?;

    // Set file permissions to 600 (owner read/write).
    // This is a sensitive operation and might fail if not run with sufficient privileges.
    // On illumos, this will require the process to have the `file_chown_self` privilege.
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(cert_path, perms.clone())?;
    fs::set_permissions(key_path, perms)?;

    info!("Successfully generated and saved TLS certificate and key.");

    Ok(())
}
