//! Tests for the error handling module.
//!
//! These tests verify that AppError variants correctly map to
//! HTTP status codes and JSON error responses.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use http_body_util::BodyExt;
use std::io::{Error as IoError, ErrorKind};

use yanos_backend::error::{AppError, ErrorResponse};

/// Test InternalServerError returns 500.
#[tokio::test]
async fn test_internal_server_error() {
    let error = AppError::InternalServerError("Database connection failed".to_string());
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, "Database connection failed");
}

/// Test IoError returns 500 with generic message.
#[tokio::test]
async fn test_io_error() {
    let io_err = IoError::new(ErrorKind::NotFound, "file not found");
    let error = AppError::IoError(io_err);
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    // Should NOT leak the actual error message
    assert_eq!(error_response.error, "An internal I/O error occurred");
}

/// Test Unauthorized returns 401.
#[tokio::test]
async fn test_unauthorized_error() {
    let error = AppError::Unauthorized("Invalid credentials".to_string());
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, "Invalid credentials");
}

/// Test ServiceUnavailable returns 503.
#[tokio::test]
async fn test_service_unavailable_error() {
    let error = AppError::ServiceUnavailable("ZFS pool offline".to_string());
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, "ZFS pool offline");
}

/// Test BadRequest returns 400.
#[tokio::test]
async fn test_bad_request_error() {
    let error = AppError::BadRequest("Invalid pool name".to_string());
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, "Invalid pool name");
}

/// Test NotFound returns 404.
#[tokio::test]
async fn test_not_found_error() {
    let error = AppError::NotFound("Pool 'tank' not found".to_string());
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, "Pool 'tank' not found");
}

/// Test From<std::io::Error> implementation.
#[test]
fn test_io_error_conversion() {
    let io_err = IoError::new(ErrorKind::PermissionDenied, "access denied");
    let app_err: AppError = io_err.into();

    match app_err {
        AppError::IoError(e) => {
            assert_eq!(e.kind(), ErrorKind::PermissionDenied);
        }
        _ => panic!("Expected IoError variant"),
    }
}

/// Test ErrorResponse serialization.
#[test]
fn test_error_response_serialization() {
    let response = ErrorResponse {
        error: "Test error message".to_string(),
    };

    let json = serde_json::to_string(&response).expect("Serialization failed");
    assert!(json.contains("\"error\":\"Test error message\""));

    let deserialized: ErrorResponse = serde_json::from_str(&json)
        .expect("Deserialization failed");
    assert_eq!(deserialized.error, "Test error message");
}

/// Test AppError Debug implementation.
#[test]
fn test_app_error_debug() {
    let error = AppError::InternalServerError("test".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("InternalServerError"));
    assert!(debug_str.contains("test"));
}

/// Test that all error variants produce valid JSON.
#[tokio::test]
async fn test_all_variants_produce_valid_json() {
    let variants: Vec<AppError> = vec![
        AppError::InternalServerError("internal".to_string()),
        AppError::IoError(IoError::new(ErrorKind::Other, "io")),
        AppError::Unauthorized("unauth".to_string()),
        AppError::ServiceUnavailable("unavail".to_string()),
        AppError::BadRequest("bad".to_string()),
        AppError::NotFound("notfound".to_string()),
    ];

    for error in variants {
        let response = error.into_response();
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();

        // Should be valid JSON
        let result: Result<ErrorResponse, _> = serde_json::from_slice(&body_bytes);
        assert!(result.is_ok(), "Error response should be valid JSON");
    }
}

// --- Edge Case Tests ---

/// Test error with empty message.
#[tokio::test]
async fn test_error_empty_message() {
    let error = AppError::InternalServerError("".to_string());
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, "");
}

/// Test error with very long message.
#[tokio::test]
async fn test_error_long_message() {
    let long_msg = "x".repeat(10000);
    let error = AppError::BadRequest(long_msg.clone());
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, long_msg);
}

/// Test error with special characters in message.
#[tokio::test]
async fn test_error_special_characters() {
    let special_msg = "Error: \"file\" not found\nPath: /etc/test\t<script>";
    let error = AppError::NotFound(special_msg.to_string());
    let response = error.into_response();

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, special_msg);
}

/// Test error with unicode characters.
#[tokio::test]
async fn test_error_unicode_message() {
    let unicode_msg = "Fehler: Datei nicht gefunden";
    let error = AppError::NotFound(unicode_msg.to_string());
    let response = error.into_response();

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(error_response.error, unicode_msg);
}

/// Test IoError with various error kinds.
#[tokio::test]
async fn test_io_error_kinds() {
    let error_kinds = [
        ErrorKind::NotFound,
        ErrorKind::PermissionDenied,
        ErrorKind::ConnectionRefused,
        ErrorKind::TimedOut,
        ErrorKind::Interrupted,
        ErrorKind::OutOfMemory,
    ];

    for kind in error_kinds {
        let io_err = IoError::new(kind, "test error");
        let app_err = AppError::IoError(io_err);
        let response = app_err.into_response();

        // All IO errors should map to 500
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Message should be generic (not leak internal details)
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(error_response.error, "An internal I/O error occurred");
    }
}
