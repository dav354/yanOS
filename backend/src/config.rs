//! Application configuration management.
//!
//! Handles loading and persisting yanOS configuration from JSON files.
//! Configuration is intentionally minimal - most system state is read
//! directly from the OS (following "system files as source of truth").
//!
//! # Default Location
//! `/etc/opt/yanos/config.json`
//!
//! # Telemetry Settings
//! Configure external receivers for telemetry data:
//! - `telemetry.tempo_endpoint` - Tempo endpoint for traces (OTLP/gRPC)
//! - `telemetry.loki_endpoint` - Loki endpoint for logs (OTLP/gRPC)
//! - `telemetry.prometheus_endpoint` - Prometheus remote write endpoint

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::error::AppError;

/// Default path for the application configuration file
pub const DEFAULT_CONFIG_PATH: &str = "/etc/opt/yanos/config.json";

/// Telemetry/observability configuration.
///
/// Configure external receivers for telemetry data export.
/// Each endpoint is independent - leave empty to disable that export type.
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct TelemetryConfig {
    /// Tempo endpoint for distributed traces (OTLP/gRPC, e.g., "http://tempo:4317")
    /// Leave empty to disable trace export.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tempo_endpoint: Option<String>,

    /// Loki endpoint for log export (OTLP/gRPC, e.g., "http://loki:3100")
    /// Leave empty to disable log export.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loki_endpoint: Option<String>,

    /// Prometheus remote write endpoint (e.g., "http://prometheus:9090/api/v1/write")
    /// Leave empty to disable metrics export.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prometheus_endpoint: Option<String>,
}

impl TelemetryConfig {
    /// Get the Tempo endpoint for traces.
    pub fn get_tempo_endpoint(&self) -> Option<&String> {
        self.tempo_endpoint.as_ref()
    }

    /// Get the Loki endpoint for logs.
    pub fn get_loki_endpoint(&self) -> Option<&String> {
        self.loki_endpoint.as_ref()
    }

    /// Get the Prometheus endpoint for metrics.
    pub fn get_prometheus_endpoint(&self) -> Option<&String> {
        self.prometheus_endpoint.as_ref()
    }

    /// Returns true if any telemetry export is configured.
    pub fn is_enabled(&self) -> bool {
        self.tempo_endpoint.is_some()
            || self.loki_endpoint.is_some()
            || self.prometheus_endpoint.is_some()
    }
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
