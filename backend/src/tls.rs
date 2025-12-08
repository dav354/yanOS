// backend/src/tls.rs

use axum_server::tls_rustls::RustlsConfig;
use rcgen::generate_simple_self_signed;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};
use tokio::time::{self, Duration as TokioDuration};
use tracing::{error, info};

pub const DEFAULT_TLS_DIR: &str = "/etc/opt/storage-os/tls";

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
        "zos".to_string(),
        "zos.local".to_string(),
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

fn mtimes(cert_path: &Path, key_path: &Path) -> io::Result<(SystemTime, SystemTime)> {
    let cert_meta = fs::metadata(cert_path)?;
    let key_meta = fs::metadata(key_path)?;
    let cert_mtime = cert_meta.modified()?;
    let key_mtime = key_meta.modified()?;
    Ok((cert_mtime, key_mtime))
}

/// Tracks TLS config and reloads it when files change.
#[derive(Clone, Debug)]
pub struct TlsState {
    cert_path: PathBuf,
    key_path: PathBuf,
    config: RustlsConfig,
    last_reload_ok: Arc<AtomicBool>,
}

impl TlsState {
    pub async fn load(cert_dir: &Path) -> io::Result<Self> {
        ensure_tls_certs_exist(cert_dir)?;

        let cert_path = cert_dir.join("cert.pem");
        let key_path = cert_dir.join("key.pem");
        let config = RustlsConfig::from_pem_file(&cert_path, &key_path).await?;

        Ok(Self {
            cert_path,
            key_path,
            config,
            last_reload_ok: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn config(&self) -> RustlsConfig {
        self.config.clone()
    }

    pub fn is_ready(&self) -> bool {
        self.last_reload_ok.load(Ordering::SeqCst)
    }

    pub fn spawn_reload_task(&self) {
        let cert_path = self.cert_path.clone();
        let key_path = self.key_path.clone();
        let config = self.config.clone();
        let reload_status = self.last_reload_ok.clone();
        let mut previous_mtimes = mtimes(&cert_path, &key_path).ok();

        tokio::spawn(async move {
            let mut interval = time::interval(TokioDuration::from_secs(30));
            loop {
                interval.tick().await;

                match mtimes(&cert_path, &key_path) {
                    Ok(current) => {
                        if Some(current) != previous_mtimes {
                            match config.reload_from_pem_file(&cert_path, &key_path).await {
                                Ok(_) => {
                                    reload_status.store(true, Ordering::SeqCst);
                                    previous_mtimes = Some(current);
                                    info!(target: "zos::tls", "Reloaded TLS certificate and key");
                                }
                                Err(err) => {
                                    reload_status.store(false, Ordering::SeqCst);
                                    error!(target: "zos::tls", error = ?err, "Failed to reload TLS material");
                                }
                            }
                        }
                    }
                    Err(err) => {
                        reload_status.store(false, Ordering::SeqCst);
                        error!(target: "zos::tls", error = ?err, "Failed to read TLS metadata");
                    }
                }
            }
        });
    }
}
