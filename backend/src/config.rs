//! Application configuration management.
//!
//! Handles loading and persisting yanOS configuration from JSON files.
//! Configuration is intentionally minimal - most system state is read
//! directly from the OS (following "system files as source of truth").
//!
//! # Default Location
//! `/etc/opt/yanos/config.json`
//!
//! # Current Settings
//! - `telemetry.otlp_endpoint` - Optional OpenTelemetry OTLP endpoint URL

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::AppError;

/// Default path for the application configuration file
pub const DEFAULT_CONFIG_PATH: &str = "/etc/opt/yanos/config.json";

/// Telemetry/observability configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryConfig {
    /// OpenTelemetry OTLP collector endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,
}

/// Root application configuration.
/// Add new settings here as the application grows.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Telemetry settings (optional)
    pub telemetry: TelemetryConfig,
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref();
        match fs::read(path) {
            Ok(bytes) => {
                let cfg: AppConfig = serde_json::from_slice(&bytes).map_err(|e| {
                    AppError::InternalServerError(format!(
                        "Failed to parse config at {}: {e}",
                        path.display()
                    ))
                })?;
                Ok(cfg)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                warn!(
                    target: "yanos::config",
                    path = %path.display(),
                    "Config file missing, using defaults"
                );
                Ok(AppConfig::default())
            }
            Err(err) => Err(AppError::IoError(err)),
        }
    }

    pub fn persist(&self, path: impl AsRef<Path>) -> Result<(), AppError> {
        let path: PathBuf = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let serialized = serde_json::to_vec_pretty(self).map_err(|e| {
            AppError::InternalServerError(format!("Failed to serialize config: {e}"))
        })?;
        fs::write(&path, serialized)?;

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            if let Err(err) = fs::set_permissions(&path, perms) {
                warn!(
                    target: "yanos::config",
                    path = %path.display(),
                    error = ?err,
                    "Failed to set restrictive permissions on config file"
                );
            }
        }

        info!(
            target: "yanos::config",
            path = %path.display(),
            "Persisted configuration"
        );
        Ok(())
    }
}
