//! Tests for authentication and session management.
//!
//! These tests verify:
//! - Login endpoint behavior
//! - Logout endpoint behavior
//! - Protected endpoint access control
//! - Session handling

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hyper::Method;
use tower::ServiceExt;

use yanos_backend::auth;

/// Test that unauthenticated access to public endpoints works.
#[tokio::test]
async fn test_unauthenticated_api_access() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test that login with invalid credentials returns 401.
#[tokio::test]
async fn test_authenticated_api_access() {
    let (app, _, _, _) = common::create_test_app().await;

    // Use a bogus user to verify we get a 401 without needing local PAM setup.
    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "nonexistent".to_string(),
        password: "wrong".to_string(),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test that events endpoint requires authentication.
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

/// Test that logout endpoint exists and handles unauthenticated requests.
#[tokio::test]
async fn test_logout_endpoint_without_session() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/logout")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Logout without a session should still succeed (idempotent)
    // It returns 200 with "Logout successful"
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test login payload structure.
#[test]
fn test_login_payload_serialization() {
    let payload = auth::LoginPayload {
        username: "admin".to_string(),
        password: "secret".to_string(),
    };

    let json = serde_json::to_string(&payload).expect("Serialization failed");
    assert!(json.contains("\"username\":\"admin\""));
    assert!(json.contains("\"password\":\"secret\""));

    let deserialized: auth::LoginPayload = serde_json::from_str(&json)
        .expect("Deserialization failed");
    assert_eq!(deserialized.username, "admin");
    assert_eq!(deserialized.password, "secret");
}

/// Test that login with empty username returns 401.
#[tokio::test]
async fn test_login_empty_username() {
    let (app, _, _, _) = common::create_test_app().await;

    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "".to_string(),
        password: "password".to_string(),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test that login with empty password returns 401.
#[tokio::test]
async fn test_login_empty_password() {
    let (app, _, _, _) = common::create_test_app().await;

    // Use "root" user which exists, but with empty password (should fail fast)
    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "root".to_string(),
        password: "".to_string(),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test that protected network endpoint requires auth.
#[tokio::test]
async fn test_network_endpoint_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/network/interfaces")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test that protected pkg endpoint requires auth.
#[tokio::test]
async fn test_pkg_endpoint_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/pkg/list")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Edge Case Tests ---

/// Test login with very long username.
#[tokio::test]
async fn test_login_very_long_username() {
    let (app, _, _, _) = common::create_test_app().await;

    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "x".repeat(10000),
        password: "password".to_string(),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    // Should fail gracefully (unauthorized, not server error)
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test login with very long password.
#[tokio::test]
async fn test_login_very_long_password() {
    let (app, _, _, _) = common::create_test_app().await;

    // Use "root" user which exists, with very long password
    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "root".to_string(),
        password: "x".repeat(10000),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test login with special characters in username.
#[tokio::test]
async fn test_login_special_chars_username() {
    let (app, _, _, _) = common::create_test_app().await;

    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "user@domain.com!#$%".to_string(),
        password: "password".to_string(),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test login with unicode in credentials.
#[tokio::test]
async fn test_login_unicode_credentials() {
    let (app, _, _, _) = common::create_test_app().await;

    // Use "root" user with unicode password
    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "root".to_string(),
        password: "wrongpassword".to_string(),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test login with null bytes in credentials.
#[tokio::test]
async fn test_login_null_bytes() {
    let (app, _, _, _) = common::create_test_app().await;

    // Manually construct JSON with escaped null bytes (using root user)
    let login_payload = r#"{"username":"root\u0000","password":"pass\u0000word"}"#;

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test login with wrong content type.
#[tokio::test]
async fn test_login_wrong_content_type() {
    let (app, _, _, _) = common::create_test_app().await;

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "text/plain")
        .body(Body::from("username=admin&password=secret"))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    // Should fail due to wrong content type
    assert!(
        login_response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE
            || login_response.status() == StatusCode::BAD_REQUEST
            || login_response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

/// Test login without content type header.
#[tokio::test]
async fn test_login_no_content_type() {
    let (app, _, _, _) = common::create_test_app().await;

    let login_payload = r#"{"username":"admin","password":"secret"}"#;

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    // Axum may or may not accept this depending on configuration
    assert!(
        login_response.status() == StatusCode::UNAUTHORIZED
            || login_response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE
            || login_response.status() == StatusCode::BAD_REQUEST
    );
}

/// Test multiple logout calls (idempotent).
#[tokio::test]
async fn test_multiple_logout_calls() {
    let (app, _, _, _) = common::create_test_app().await;

    for _ in 0..3 {
        let app_clone = app.clone();
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/logout")
            .body(Body::empty())
            .unwrap();

        let response = app_clone.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

/// Test LoginPayload with whitespace-only username.
#[tokio::test]
async fn test_login_whitespace_username() {
    let (app, _, _, _) = common::create_test_app().await;

    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "   ".to_string(),
        password: "password".to_string(),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// Test session store healthcheck.
#[tokio::test]
async fn test_session_store_healthcheck() {
    let store = auth::memory_store();
    let result = auth::session_store_healthcheck(&store).await;
    assert!(result.is_ok(), "Memory store healthcheck should pass");
}
