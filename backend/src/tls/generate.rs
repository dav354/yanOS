use std::{fs, io, path::Path};

use rcgen::generate_simple_self_signed;
use tracing::info;

/// Ensures that TLS certificate and key exist, generating them if they don't.
pub fn ensure_tls_certs_exist(cert_dir: &Path) -> io::Result<()> {
    let cert_path = cert_dir.join("cert.pem");
    let key_path = cert_dir.join("key.pem");

    if cert_path.exists() && key_path.exists() {
        info!("TLS certificate and key found at {:?}.", cert_dir);
        return Ok(());
    }

    info!(
        "TLS certificate or key not found at {:?}. Generating self-signed certificate...",
        cert_dir
    );

    // Create the directory if it doesn't exist.
    fs::create_dir_all(cert_dir)?;

    // Generate a new self-signed certificate.
    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "yanos".to_string(),
        "yanos.local".to_string(),
    ];
    let certified_key = generate_simple_self_signed(subject_alt_names).map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to generate self-signed certificate: {err}"),
        )
    })?;
    let cert_pem = certified_key.cert.pem();
    let key_pem = certified_key.signing_key.serialize_pem();

    // Write the certificate and key to their respective files.
    fs::write(&cert_path, cert_pem)?;
    fs::write(&key_path, key_pem)?;

    // Set file permissions to 600 (owner read/write).
    // This is a sensitive operation and might fail if not run with sufficient privileges.
    // On illumos, this will require the process to have the `file_chown_self` privilege.
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(&cert_path, perms.clone())?;
    fs::set_permissions(&key_path, perms)?;

    info!("Successfully generated and saved TLS certificate and key.");

    Ok(())
}
