mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hyper::Method;
use tower::ServiceExt;

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

#[tokio::test]
async fn test_network_interfaces_endpoint() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/network/interfaces")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_pkg_list_endpoint() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/pkg/list")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
