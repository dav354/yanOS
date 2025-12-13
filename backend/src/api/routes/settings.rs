use axum::{Json, Router, extract::State, routing::{get, post}};
use tracing::instrument;
use utoipa::ToSchema;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use axum::http::Uri;

use crate::api::AppState;
use crate::config::{AppConfig, TelemetryConfig};
use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TelemetrySettings {
    pub otlp_endpoint: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/settings/telemetry", get(get_telemetry).put(update_telemetry))
        .route("/settings/telemetry/test", post(test_telemetry))
}

#[utoipa::path(
    get,
    path = "/api/v1/settings/telemetry",
    tag = "settings",
    responses(
        (status = 200, description = "Current telemetry settings", body = TelemetrySettings)
    ),
    security(
        ("basic_auth" = [])
    )
)]
#[instrument(skip(state))]
pub async fn get_telemetry(State(state): State<AppState>) -> Result<Json<TelemetrySettings>, AppError> {
    let cfg = AppConfig::load(&state.config_path)?;
    Ok(Json(TelemetrySettings {
        otlp_endpoint: cfg.telemetry.otlp_endpoint,
    }))
}

#[utoipa::path(
    put,
    path = "/api/v1/settings/telemetry",
    tag = "settings",
    request_body = TelemetrySettings,
    responses(
        (status = 200, description = "Telemetry settings updated", body = TelemetrySettings)
    ),
    security(
        ("basic_auth" = [])
    )
)]
#[instrument(skip(state, payload))]
pub async fn update_telemetry(
    State(state): State<AppState>,
    Json(payload): Json<TelemetrySettings>,
) -> Result<Json<TelemetrySettings>, AppError> {
    let mut cfg = AppConfig::load(&state.config_path).unwrap_or_default();
    cfg.telemetry = TelemetryConfig {
        otlp_endpoint: payload
            .otlp_endpoint
            .and_then(|v| {
                let trimmed = v.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }),
    };

    cfg.persist(&state.config_path)?;

    Ok(Json(TelemetrySettings {
        otlp_endpoint: cfg.telemetry.otlp_endpoint,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/telemetry/test",
    tag = "settings",
    request_body = TelemetrySettings,
    responses(
        (status = 200, description = "Endpoint reachable"),
        (status = 503, description = "Endpoint not reachable or invalid")
    ),
    security(
        ("basic_auth" = [])
    )
)]
#[instrument(skip(payload))]
pub async fn test_telemetry(
    Json(payload): Json<TelemetrySettings>,
) -> Result<Json<TelemetrySettings>, AppError> {
    let endpoint = payload.otlp_endpoint
        .and_then(|s| {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        })
        .ok_or_else(|| AppError::ServiceUnavailable("OTLP endpoint is required".to_string()))?;

    let uri: Uri = endpoint
        .parse()
        .map_err(|e| AppError::ServiceUnavailable(format!("Invalid OTLP endpoint: {e}")))?;

    let host = uri.host().ok_or_else(|| {
        AppError::ServiceUnavailable("OTLP endpoint missing host".to_string())
    })?;
    let port = uri
        .port_u16()
        .or_else(|| match uri.scheme_str() {
            Some("https") => Some(443),
            Some("http") => Some(80),
            _ => None,
        })
        .ok_or_else(|| AppError::ServiceUnavailable("OTLP endpoint missing port".to_string()))?;

    let target = format!("{host}:{port}");

    match timeout(Duration::from_secs(3), TcpStream::connect(target)).await {
        Ok(Ok(_)) => Ok(Json(TelemetrySettings {
            otlp_endpoint: Some(endpoint),
        })),
        Ok(Err(e)) => Err(AppError::ServiceUnavailable(format!("Failed to connect: {e}"))),
        Err(_) => Err(AppError::ServiceUnavailable(
            "Timed out connecting to OTLP endpoint".to_string(),
        )),
    }
}
