mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hyper::Method;
use tower::ServiceExt;

use yanos_backend::{api, auth};

#[tokio::test]
async fn test_healthz_returns_ok() {
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

#[tokio::test]
async fn test_readyz_when_tls_present() {
    let (app, _, _, temp_dir_tls) = common::create_test_app().await;

    // Ensure TLS certs exist for this test (they are created by create_test_app)
    assert!(temp_dir_tls.path().join("cert.pem").exists());
    assert!(temp_dir_tls.path().join("key.pem").exists());

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

#[tokio::test]
async fn test_readyz_fails_when_session_store_is_unhealthy() {
    let (_, app_state, temp_dir_session, _) = common::create_test_app().await;
    let failing_store = auth::DynSessionStore::new(common::FailingStore);

    let new_app_state_for_test = api::AppState::new(
        app_state.csrf_config.clone(),
        failing_store,
        app_state.tls_state.clone(),
        app_state.event_bus.clone(),
        app_state.network_actor.clone(),
        app_state.pkg_actor.clone(),
        app_state.metrics_state.clone(),
        temp_dir_session.path().join("config.json"),
    );
    let app_with_failing_session = api::create_router().with_state(new_app_state_for_test);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/readyz")
        .body(Body::empty())
        .unwrap();

    let response = app_with_failing_session.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json_body: serde_json::Value = common::get_body_as_json(response).await;
    assert!(
        json_body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Session store create failed")
    );
}
