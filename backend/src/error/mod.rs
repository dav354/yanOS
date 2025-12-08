// backend/src/error.rs

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use tracing::error;

/// A unified error type for the application.
#[derive(Debug)]
pub enum AppError {
    /// Used when an internal server error occurs.
    InternalServerError(String),
    /// Used for errors related to I/O operations.
    IoError(std::io::Error),
    /// Used when authentication fails.
    Unauthorized(String),
    /// Used when the service is not ready.
    ServiceUnavailable(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::InternalServerError(msg) => {
                error!("Internal Server Error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            AppError::IoError(err) => {
                error!("I/O Error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal I/O error occurred".to_string(),
                )
            }
            AppError::Unauthorized(msg) => {
                error!("Unauthorized: {}", msg);
                (StatusCode::UNAUTHORIZED, msg)
            }
            AppError::ServiceUnavailable(msg) => {
                error!("Service Unavailable: {}", msg);
                (StatusCode::SERVICE_UNAVAILABLE, msg)
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

/// Allows converting `std::io::Error` into `AppError`.
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err)
    }
}
