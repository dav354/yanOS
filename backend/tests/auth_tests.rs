mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hyper::Method;
use tower::ServiceExt;

use zos_backend::auth;

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
