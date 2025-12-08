use std::path::Path;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use async_trait::async_trait;
use http_body_util::BodyExt; // for `collect` and `to_bytes`
use hyper::Method;
use tower::ServiceExt; // for `call`, `ready`
use tower_sessions::SessionStore;
use tower_sessions_core::session::{Id, Record};
use tower_sessions_core::session_store;

// Import the necessary modules from our crate
use serde_json::Value;
use zos_backend::{api, auth, tls};

// Helper to create a test app
async fn create_test_app(cert_dir: &Path) -> Router {
    let csrf_config = auth::create_csrf_config();

    // Create temporary directory for session key
    let temp_dir_session =
        tempfile::tempdir().expect("Failed to create temporary directory for session key");
    let session_key_path = temp_dir_session.path().join("session.key");

    let (session_layer, session_store) =
        auth::create_session_manager_layer_for_test(&session_key_path);

    let tls_state = tls::TlsState::load(cert_dir)
        .await
        .expect("Failed to load TLS state for test");
    let app_state = api::AppState::new(csrf_config.clone(), session_store, tls_state);

    let app = api::create_router(app_state);

    auth::add_auth_routes(app)
        .layer(axum_csrf::CsrfLayer::new(csrf_config))
        .layer(session_layer)
        .layer(tower_cookies::CookieManagerLayer::new())
}

// Helper to extract response body
async fn get_body_as_json(response: axum::response::Response) -> Value {
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body_bytes).unwrap()
}

// --- Auth Guard Tests ---

#[tokio::test]
async fn test_unauthenticated_api_access() {
    let temp_dir_tls = tempfile::tempdir().expect("Failed to create temporary directory for TLS");
    let app = create_test_app(temp_dir_tls.path()).await;

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
    let temp_dir_tls = tempfile::tempdir().expect("Failed to create temporary directory for TLS");
    let app = create_test_app(temp_dir_tls.path()).await;

    // Use a bogus user to verify we get a 401 without needing local PAM setup.
    let login_payload = serde_json::to_string(&auth::LoginPayload {
        username: "nonexistent".to_string(),
        password: "wrong".to_string(),
    })
    .unwrap();

    let login_request = Request::builder()
        .method(Method::POST)
        .uri("/api/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload))
        .unwrap();

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

// --- Readiness Probe Tests ---

#[tokio::test]
async fn test_readyz_when_tls_present() {
    // Create temporary directory for TLS certs
    let temp_dir_tls = tempfile::tempdir().expect("Failed to create temporary directory for TLS");
    let cert_dir = temp_dir_tls.path();

    let app = create_test_app(cert_dir).await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/readyz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json_body: Value = get_body_as_json(response).await;
    assert_eq!(json_body["status"], "ready");
}

#[tokio::test]
async fn test_readyz_when_tls_missing() {
    // TLS state self-heals by creating certs, so missing TLS now results in ready.
    let temp_dir_tls = tempfile::tempdir().expect("Failed to create temporary directory for TLS");
    let cert_dir = temp_dir_tls.path();

    let app = create_test_app(cert_dir).await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/readyz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json_body: Value = get_body_as_json(response).await;
    assert_eq!(json_body["status"], "ready");
}

// --- Additional Coverage ---

#[tokio::test]
async fn test_readyz_fails_when_session_store_is_unhealthy() {
    #[derive(Clone, Debug)]
    struct FailingStore;

    #[async_trait]
    impl SessionStore for FailingStore {
        async fn create(&self, _record: &mut Record) -> session_store::Result<()> {
            Err(session_store::Error::Backend("create failed".into()))
        }

        async fn save(&self, _record: &Record) -> session_store::Result<()> {
            Err(session_store::Error::Backend("save failed".into()))
        }

        async fn load(&self, _session_id: &Id) -> session_store::Result<Option<Record>> {
            Err(session_store::Error::Backend("load failed".into()))
        }

        async fn delete(&self, _session_id: &Id) -> session_store::Result<()> {
            Err(session_store::Error::Backend("delete failed".into()))
        }
    }

    let temp_dir_tls = tempfile::tempdir().expect("Failed to create temporary directory for TLS");
    let tls_state = tls::TlsState::load(temp_dir_tls.path())
        .await
        .expect("Failed to load TLS state for test");

    let csrf_config = auth::create_csrf_config();
    let failing_store = auth::DynSessionStore::new(FailingStore);
    let app_state = api::AppState::new(csrf_config.clone(), failing_store, tls_state);
    let app = api::create_router(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/readyz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json_body: Value = get_body_as_json(response).await;
    assert!(json_body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("Session store create failed"));
}

#[tokio::test]
async fn test_api_status_returns_csrf_and_no_user() {
    let temp_dir_tls = tempfile::tempdir().expect("Failed to create temporary directory for TLS");
    let app = create_test_app(temp_dir_tls.path()).await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json_body: Value = get_body_as_json(response).await;
    let csrf = json_body["csrf_token"].as_str().unwrap_or_default();
    assert!(!csrf.is_empty(), "csrf token should be present");
    assert!(json_body["user"].is_null());
}

#[tokio::test]
async fn test_healthz_returns_ok() {
    let temp_dir_tls = tempfile::tempdir().expect("Failed to create temporary directory for TLS");
    let tls_state = tls::TlsState::load(temp_dir_tls.path())
        .await
        .expect("Failed to load TLS state for test");
    let csrf_config = auth::create_csrf_config();
    let store = auth::memory_store();
    let app_state = api::AppState::new(csrf_config.clone(), store, tls_state);
    let app = api::create_router(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/healthz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json_body: Value = get_body_as_json(response).await;
    assert_eq!(json_body["status"], "ok");
}

#[tokio::test]
async fn test_session_store_healthcheck_passes_for_memory_store() {
    let store = auth::memory_store();
    auth::session_store_healthcheck(&store)
        .await
        .expect("healthcheck should pass for memory store");
}

#[test]
fn test_session_key_is_regenerated_when_too_short() {
    let temp_dir_session =
        tempfile::tempdir().expect("Failed to create temporary directory for session key");
    let session_key_path = temp_dir_session.path().join("session.key");

    // Write an intentionally too-short key.
    std::fs::write(&session_key_path, &[1u8]).expect("Failed to write short key");

    // Building the layer will regenerate the key.
    let (_layer, _store) = auth::create_session_manager_layer_for_test(&session_key_path);

    let regenerated = std::fs::read(&session_key_path).expect("Failed to read regenerated key");
    assert!(
        regenerated.len() >= 64,
        "Regenerated key should be at least 64 bytes"
    );
}

#[tokio::test]
async fn test_tls_state_load_creates_certs_and_is_ready() {
    let temp_dir_tls = tempfile::tempdir().expect("Failed to create temporary directory for TLS");
    let tls_state = tls::TlsState::load(temp_dir_tls.path())
        .await
        .expect("Failed to load TLS state for test");

    assert!(tls_state.is_ready());
    assert!(temp_dir_tls.path().join("cert.pem").exists());
    assert!(temp_dir_tls.path().join("key.pem").exists());
}
