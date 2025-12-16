//! Settings API routes for application configuration.
//!
//! Provides endpoints for managing telemetry receiver configuration.
//! Configure where to export traces (Tempo), logs (Loki), and metrics (Prometheus).

use axum::http::Uri;
use axum::{extract::State, routing::get, Json, Router};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::{debug, instrument};
use utoipa::ToSchema;

use crate::api::AppState;
use crate::config::{AppConfig, TelemetryConfig};
use crate::error::AppError;

/// Telemetry receiver settings.
///
/// Configure external services to receive telemetry data.
/// Each endpoint is independent - leave empty to disable that export type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TelemetrySettings {
    /// Tempo endpoint for distributed traces (OTLP/gRPC, e.g., "http://tempo:4317")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tempo_endpoint: Option<String>,

    /// Loki endpoint for logs (OTLP/gRPC, e.g., "http://loki:3100")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loki_endpoint: Option<String>,

    /// Prometheus remote write endpoint (e.g., "http://prometheus:9090/api/v1/write")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prometheus_endpoint: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/settings/telemetry", get(get_telemetry).put(update_telemetry))
        .route("/settings/telemetry/test", axum::routing::post(test_endpoint))
}

/// Get current telemetry receiver settings.
#[utoipa::path(
    get,
    path = "/api/v1/settings/telemetry",
    tag = "settings",
    responses(
        (status = 200, description = "Current telemetry receiver settings", body = TelemetrySettings)
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_telemetry(
    State(state): State<AppState>,
) -> Result<Json<TelemetrySettings>, AppError> {
    debug!(target: "yanos::api", "GET /settings/telemetry");
    let cfg = AppConfig::load(&state.config_path)?;
    Ok(Json(TelemetrySettings {
        tempo_endpoint: cfg.telemetry.tempo_endpoint,
        loki_endpoint: cfg.telemetry.loki_endpoint,
        prometheus_endpoint: cfg.telemetry.prometheus_endpoint,
    }))
}

/// Update telemetry receiver settings.
///
/// Configure where to export traces (Tempo), logs (Loki), and metrics (Prometheus).
/// Each endpoint is independent - leave empty/null to disable that export type.
/// The server must be restarted for changes to take effect.
#[utoipa::path(
    put,
    path = "/api/v1/settings/telemetry",
    tag = "settings",
    request_body = TelemetrySettings,
    responses(
        (status = 200, description = "Telemetry receiver settings updated", body = TelemetrySettings)
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(state, payload))]
pub async fn update_telemetry(
    State(state): State<AppState>,
    Json(payload): Json<TelemetrySettings>,
) -> Result<Json<TelemetrySettings>, AppError> {
    debug!(target: "yanos::api", ?payload, "PUT /settings/telemetry");

    // Normalize empty strings to None
    let tempo = payload
        .tempo_endpoint
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let loki = payload
        .loki_endpoint
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let prometheus = payload
        .prometheus_endpoint
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Save configuration
    let mut cfg = AppConfig::load(&state.config_path).unwrap_or_default();
    cfg.telemetry = TelemetryConfig {
        tempo_endpoint: tempo.clone(),
        loki_endpoint: loki.clone(),
        prometheus_endpoint: prometheus.clone(),
    };

    cfg.persist(&state.config_path)?;

    debug!(target: "yanos::api", "Telemetry receiver settings saved");
    Ok(Json(TelemetrySettings {
        tempo_endpoint: tempo,
        loki_endpoint: loki,
        prometheus_endpoint: prometheus,
    }))
}

/// Request to test an OTLP endpoint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TestEndpointRequest {
    /// The endpoint URL to test (e.g., "http://collector:4317")
    pub endpoint: String,
}

/// Response from endpoint test.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TestEndpointResponse {
    pub reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Test if an OTLP endpoint is reachable.
#[utoipa::path(
    post,
    path = "/api/v1/settings/telemetry/test",
    tag = "settings",
    request_body = TestEndpointRequest,
    responses(
        (status = 200, description = "Endpoint test result", body = TestEndpointResponse)
    ),
    security(("basic_auth" = []))
)]
#[instrument(skip(payload))]
pub async fn test_endpoint(
    Json(payload): Json<TestEndpointRequest>,
) -> Result<Json<TestEndpointResponse>, AppError> {
    let endpoint = payload.endpoint.trim();
    debug!(target: "yanos::api", endpoint = %endpoint, "POST /settings/telemetry/test");

    if endpoint.is_empty() {
        return Ok(Json(TestEndpointResponse {
            reachable: false,
            error: Some("Endpoint is empty".to_string()),
        }));
    }

    // Parse the URI
    let uri: Uri = match endpoint.parse() {
        Ok(u) => u,
        Err(e) => {
            return Ok(Json(TestEndpointResponse {
                reachable: false,
                error: Some(format!("Invalid URL: {e}")),
            }));
        }
    };

    let host = match uri.host() {
        Some(h) => h,
        None => {
            return Ok(Json(TestEndpointResponse {
                reachable: false,
                error: Some("URL missing host".to_string()),
            }));
        }
    };

    let port = uri.port_u16().unwrap_or_else(|| match uri.scheme_str() {
        Some("https") => 443,
        Some("http") => 80,
        _ => 4317, // Default OTLP port
    });

    let target = format!("{host}:{port}");

    // Test connectivity
    match timeout(Duration::from_secs(3), TcpStream::connect(&target)).await {
        Ok(Ok(_)) => {
            debug!(target: "yanos::api", endpoint = %endpoint, "Endpoint reachable");
            Ok(Json(TestEndpointResponse {
                reachable: true,
                error: None,
            }))
        }
        Ok(Err(e)) => {
            debug!(target: "yanos::api", endpoint = %endpoint, error = %e, "Endpoint unreachable");
            Ok(Json(TestEndpointResponse {
                reachable: false,
                error: Some(format!("Connection failed: {e}")),
            }))
        }
        Err(_) => {
            debug!(target: "yanos::api", endpoint = %endpoint, "Endpoint timeout");
            Ok(Json(TestEndpointResponse {
                reachable: false,
                error: Some("Connection timed out".to_string()),
            }))
        }
    }
}
