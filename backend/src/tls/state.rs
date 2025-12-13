//! TLS certificate management with automatic reload.
//!
//! This module handles TLS configuration for the HTTPS server:
//! - Loads certificates from disk (or generates self-signed on first run)
//! - Monitors certificate files for changes every 30 seconds
//! - Hot-reloads certificates without server restart
//!
//! # Certificate Location
//! By default, certificates are stored in `/etc/opt/yanos/tls/`:
//! - `cert.pem` - PEM-encoded certificate chain
//! - `key.pem` - PEM-encoded private key
//!
//! # ALPN Configuration
//! Forces HTTP/1.1 ALPN to ensure WebSocket upgrades work correctly
//! (HTTP/2 doesn't support WebSocket in the same way).

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};

use axum_server::tls_rustls::RustlsConfig;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use rustls_pki_types::pem::PemObject;
use tokio::time::{self, Duration as TokioDuration};
use tracing::{error, info};

use crate::tls::generate::ensure_tls_certs_exist;

/// Default directory for TLS certificates
pub const DEFAULT_TLS_DIR: &str = "/etc/opt/yanos/tls";

/// Get modification times for cert and key files (used for change detection).
fn mtimes(cert_path: &Path, key_path: &Path) -> io::Result<(SystemTime, SystemTime)> {
    let cert_meta = fs::metadata(cert_path)?;
    let key_meta = fs::metadata(key_path)?;
    let cert_mtime = cert_meta.modified()?;
    let key_mtime = key_meta.modified()?;
    Ok((cert_mtime, key_mtime))
}

/// TLS state manager with automatic certificate reload.
///
/// Holds the RustlsConfig and spawns a background task to poll for
/// certificate file changes. When changes are detected, reloads the
/// certificate/key pair without requiring a server restart.
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
        let config = build_rustls_config(&cert_path, &key_path).await?;

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
                            match build_rustls_config(&cert_path, &key_path).await {
                                Ok(new_cfg) => {
                                    config.reload_from_config(new_cfg.get_inner());
                                    reload_status.store(true, Ordering::SeqCst);
                                    previous_mtimes = Some(current);
                                    info!(target: "yanos::tls", "Reloaded TLS certificate and key");
                                }
                                Err(err) => {
                                    reload_status.store(false, Ordering::SeqCst);
                                    error!(target: "yanos::tls", error = ?err, "Failed to reload TLS material");
                                }
                            }
                        }
                    }
                    Err(err) => {
                        reload_status.store(false, Ordering::SeqCst);
                        error!(target: "yanos::tls", error = ?err, "Failed to read TLS metadata");
                    }
                }
            }
        });
    }
}

async fn build_rustls_config(cert_path: &Path, key_path: &Path) -> io::Result<RustlsConfig> {
    let cert_bytes = tokio::fs::read(cert_path).await?;
    let key_bytes = tokio::fs::read(key_path).await?;

    let certs: Vec<CertificateDer> = CertificateDer::pem_slice_iter(&cert_bytes)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid certificate pem"))?;

    let mut key_result: Result<PrivateKeyDer, io::Error> =
        Err(io::Error::new(io::ErrorKind::InvalidData, "missing private key"));

    for item in rustls_pki_types::pem::PemObject::pem_slice_iter(&key_bytes) {
        let key: Result<PrivateKeyDer, io::Error> =
            item.map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid key pem"));
        match key_result {
            Ok(_) => {
                if key.is_ok() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "multiple private keys found",
                    ));
                }
            }
            Err(_) => key_result = key,
        }
    }

    let key = key_result?;

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid cert/key: {e}")))?;

    // Force HTTP/1.1 ALPN to keep WebSocket upgrades working
    server_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    Ok(RustlsConfig::from_config(Arc::new(server_config)))
}
