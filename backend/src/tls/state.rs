use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::SystemTime,
};

use axum_server::tls_rustls::RustlsConfig;
use tokio::time::{self, Duration as TokioDuration};
use tracing::{error, info};

use crate::tls::generate::ensure_tls_certs_exist;

pub const DEFAULT_TLS_DIR: &str = "/etc/opt/yanos/tls";

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
