//! Unified error handling for the yanOS backend.
//!
//! This module provides a single `AppError` enum that maps to HTTP status codes
//! and JSON error responses. All handlers should return `Result<T, AppError>`.
//!
//! # Error Mapping
//! - `InternalServerError` -> 500
//! - `IoError` -> 500 (generic message to avoid leaking internals)
//! - `Unauthorized` -> 401
//! - `ServiceUnavailable` -> 503
//! - `BadRequest` -> 400
//! - `NotFound` -> 404

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;

/// JSON response body for API errors.
/// Returned with appropriate HTTP status codes.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

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
    /// Used for invalid request parameters or body.
    BadRequest(String),
    /// Used when a requested resource is not found.
    NotFound(String),
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
            AppError::BadRequest(msg) => {
                error!("Bad Request: {}", msg);
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::NotFound(msg) => {
                error!("Not Found: {}", msg);
                (StatusCode::NOT_FOUND, msg)
            }
        };

        let body = Json(ErrorResponse {
            error: error_message,
        });

        (status, body).into_response()
    }
}

/// Allows converting `std::io::Error` into `AppError`.
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err)
    }
}
