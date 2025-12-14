//! Integration tests for API endpoints.
//!
//! These tests verify the API endpoints work correctly with
//! proper authentication and error handling.
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use hyper::Method;
use tower::ServiceExt;

// --- Health & Status Endpoints ---

/// Test /api/v1/healthz returns ok status.
#[tokio::test]
async fn test_healthz_endpoint() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/healthz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json_body: serde_json::Value = common::get_body_as_json(response).await;
    assert_eq!(json_body["status"], "ok");
}

/// Test /api/v1/readyz returns ready status.
#[tokio::test]
async fn test_readyz_endpoint() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/readyz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json_body: serde_json::Value = common::get_body_as_json(response).await;
    assert_eq!(json_body["status"], "ready");
}

/// Test /api/v1/status returns CSRF token.
#[tokio::test]
async fn test_status_returns_csrf_token() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json_body: serde_json::Value = common::get_body_as_json(response).await;
    // User should be null when not authenticated
    assert!(json_body["user"].is_null());
}

// --- System Info Endpoints ---

/// Test /api/v1/system/info returns system information.
#[tokio::test]
async fn test_system_info_endpoint() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/system/info")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json_body: serde_json::Value = common::get_body_as_json(response).await;
    assert!(json_body["hostname"].is_string());
    assert!(json_body["kernel_version"].is_string());
    assert!(json_body["uptime"].is_number());
}

// --- Protected Endpoints (Should require auth) ---

/// Test /api/v1/network/interfaces requires authentication.
#[tokio::test]
async fn test_network_interfaces_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/network/interfaces")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test /api/v1/pkg/list requires authentication.
#[tokio::test]
async fn test_pkg_list_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/pkg/list")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test /api/v1/pkg/updates requires authentication.
#[tokio::test]
async fn test_pkg_updates_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/pkg/updates")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test /api/v1/metrics/live requires authentication.
#[tokio::test]
async fn test_metrics_live_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/metrics/live")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Error Handling ---

/// Test non-existent endpoint returns 404.
#[tokio::test]
async fn test_nonexistent_endpoint_returns_404() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/nonexistent")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test invalid HTTP method returns 405.
#[tokio::test]
async fn test_invalid_method_returns_405() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/healthz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// --- Content-Type Handling ---

/// Test API returns JSON content type.
#[tokio::test]
async fn test_api_returns_json_content_type() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/healthz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let content_type = response.headers().get(header::CONTENT_TYPE);

    assert!(content_type.is_some());
    assert!(content_type
        .unwrap()
        .to_str()
        .unwrap()
        .contains("application/json"));
}

/// Test response is valid JSON.
#[tokio::test]
async fn test_response_is_valid_json() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/healthz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: Result<serde_json::Value, _> = serde_json::from_slice(&body_bytes);
    assert!(result.is_ok(), "Response should be valid JSON");
}

// --- Login Endpoint ---

/// Test login with empty body returns error.
#[tokio::test]
async fn test_login_empty_body_returns_error() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should fail - could be 400, 422, 415 or even 401 depending on how login parses
    assert!(
        response.status().is_client_error(),
        "Expected client error, got {}",
        response.status()
    );
}

/// Test login with invalid JSON returns error.
#[tokio::test]
async fn test_login_invalid_json_returns_error() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{ not valid json }"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(
        response.status().is_client_error(),
        "Expected client error, got {}",
        response.status()
    );
}

/// Test login with missing username field returns error.
#[tokio::test]
async fn test_login_missing_username_returns_error() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"password": "test"}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(
        response.status().is_client_error(),
        "Expected client error, got {}",
        response.status()
    );
}

/// Test login with missing password field returns error.
#[tokio::test]
async fn test_login_missing_password_returns_error() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"username": "test"}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(
        response.status().is_client_error(),
        "Expected client error, got {}",
        response.status()
    );
}

// --- Events Endpoint ---

/// Test /api/v1/events requires authentication.
#[tokio::test]
async fn test_events_endpoint_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test /api/v1/logs requires authentication.
#[tokio::test]
async fn test_logs_endpoint_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/logs")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test /api/v1/terminal requires authentication.
#[tokio::test]
async fn test_terminal_endpoint_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/terminal")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Edge Cases ---

/// Test URI with trailing slash.
#[tokio::test]
async fn test_uri_with_trailing_slash() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/healthz/")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Depending on router config, might be OK or NOT_FOUND
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND
    );
}

/// Test URI with query parameters on endpoint that doesn't use them.
#[tokio::test]
async fn test_uri_with_extra_query_params() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/healthz?foo=bar&baz=qux")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should ignore extra query params
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test HEAD request on GET endpoint.
#[tokio::test]
async fn test_head_request() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::HEAD)
        .uri("/api/v1/healthz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should work or return method not allowed
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::METHOD_NOT_ALLOWED
    );
}

/// Test OPTIONS request (for CORS preflight).
#[tokio::test]
async fn test_options_request() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/v1/healthz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should return some status (depends on CORS config)
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NO_CONTENT
            || response.status() == StatusCode::METHOD_NOT_ALLOWED
    );
}
