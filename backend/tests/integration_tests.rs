use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt; // for `collect` and `to_bytes`
use hyper::Method;
use tempfile::{TempDir, tempdir};
use tower::ServiceExt; // for `call`, `ready`
use tower_sessions::SessionStore;
use tower_sessions_core::session::{Id, Record};
use tower_sessions_core::session_store;

// Import the necessary modules from our crate
use serde_json::Value;
use zos_backend::{actors, api, auth, events::EventBus, tls};
// Helper to create a test app
async fn create_test_app() -> (Router, api::AppState, TempDir, TempDir) {
    let temp_dir_session = tempdir().expect("Failed to create temporary directory for session key");
    let session_key_path = temp_dir_session.path().join("session.key");

    let (session_layer, session_store_for_readyz_check) =
        auth::create_session_manager_layer_for_test(&session_key_path);

    let temp_dir_tls = tempdir().expect("Failed to create temporary directory for TLS");
    let cert_dir = temp_dir_tls.path();
    let tls_state = tls::TlsState::load(cert_dir)
        .await
        .expect("Failed to load TLS state");

    let csrf_config = auth::create_csrf_config();

    let event_bus = EventBus::new(8);
    let network_actor = actors::start_network_actor();
    let pkg_actor = actors::start_pkg_actor();

    let app_state = api::AppState::new(
        csrf_config.clone(),
        session_store_for_readyz_check,
        tls_state,
        event_bus,
        network_actor,
        pkg_actor,
    );

    let shared_state = app_state.clone();
    let app = api::create_router(app_state);

    let app = auth::add_auth_routes(app)
        .layer(axum_csrf::CsrfLayer::new(csrf_config))
        .layer(session_layer)
        .layer(tower_cookies::CookieManagerLayer::new());

    (app, shared_state, temp_dir_session, temp_dir_tls)
}

// Helper to extract response body
async fn get_body_as_json(response: axum::response::Response) -> Value {
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body_bytes).unwrap()
}

// --- Auth Guard Tests ---

#[tokio::test]
async fn test_unauthenticated_api_access() {
    let (app, _app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;

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
    let (app, _app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;

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
    let (app, _app_state, _temp_dir_session, temp_dir_tls) = create_test_app().await;

    // Ensure TLS certs exist for this test (they are created by create_test_app)
    assert!(temp_dir_tls.path().join("cert.pem").exists());
    assert!(temp_dir_tls.path().join("key.pem").exists());

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

    let (_app, app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;
    let failing_store = auth::DynSessionStore::new(FailingStore);

    // Create a new AppState with the failing session store
    let new_app_state_for_test = api::AppState::new(
        app_state.csrf_config.clone(),
        failing_store,
        app_state.tls_state.clone(),
        app_state.event_bus.clone(),
        app_state.network_actor.clone(),
        app_state.pkg_actor.clone(),
    );
    let app_with_failing_session = api::create_router(new_app_state_for_test);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/readyz")
        .body(Body::empty())
        .unwrap();

    let response = app_with_failing_session.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json_body: Value = get_body_as_json(response).await;
    assert!(
        json_body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Session store create failed")
    );
}

#[tokio::test]
async fn test_events_endpoint_requires_auth() {
    let (app, _app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/events")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_status_returns_csrf_and_no_user() {
    let (app, _app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json_body: Value = get_body_as_json(response).await;
    assert!(json_body["user"].is_null());
}

#[tokio::test]
async fn test_healthz_returns_ok() {
    let (app, _app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;

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
async fn test_system_info_endpoint() {
    let (app, _app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/system/info")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json_body: Value = get_body_as_json(response).await;
    assert!(json_body["hostname"].is_string());
    assert!(json_body["kernel_version"].is_string());
    assert!(json_body["uptime"].is_number());
}

#[tokio::test]
async fn test_network_interfaces_endpoint() {
    let (app, _app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/network/interfaces")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json_body: Value = get_body_as_json(response).await;
    assert!(json_body.is_array());
}

#[tokio::test]
async fn test_pkg_list_endpoint() {
    let (app, _app_state, _temp_dir_session, _temp_dir_tls) = create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/pkg/list")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json_body: Value = get_body_as_json(response).await;
    assert!(json_body.is_array());
}

#[tokio::test]
async fn test_session_store_healthcheck_passes_for_memory_store() {
    let store = auth::memory_store();
    auth::session_store_healthcheck(&store)
        .await
        .expect("healthcheck should pass for memory store");
}

#[tokio::test]
async fn test_session_key_is_regenerated_when_too_short() {
    let (_app, _app_state, temp_dir_session, _temp_dir_tls) = create_test_app().await;

    let session_key_path = temp_dir_session.path().join("session.key");
    std::fs::remove_file(&session_key_path).unwrap();

    // Write an intentionally too-short key.
    std::fs::write(&session_key_path, &[1u8]).expect("Failed to write short key");

    // Building the layer will regenerate the key.
    let (_session_layer, _session_store) =
        auth::create_session_manager_layer_for_test(&session_key_path);

    let regenerated = std::fs::read(&session_key_path).expect("Failed to read regenerated key");
    assert!(
        regenerated.len() >= 64,
        "Regenerated key should be at least 64 bytes"
    );
}

#[tokio::test]
async fn test_tls_state_load_creates_certs_and_is_ready() {
    let (_app, _app_state, _temp_dir_session, temp_dir_tls) = create_test_app().await;
    let tls_state = tls::TlsState::load(temp_dir_tls.path())
        .await
        .expect("Failed to load TLS state for test");

    assert!(tls_state.is_ready());
    assert!(temp_dir_tls.path().join("cert.pem").exists());
    assert!(temp_dir_tls.path().join("key.pem").exists());
}
