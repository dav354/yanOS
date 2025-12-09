use async_trait::async_trait;
use axum::Router;
use http_body_util::BodyExt;
use tempfile::{TempDir, tempdir};
use tower_sessions::SessionStore;
use tower_sessions_core::session::{Id, Record};
use tower_sessions_core::session_store;

use serde_json::Value;
use zos_backend::{actors, api, auth, events::EventBus, tls};

/// Build a test app with temporary session key and TLS material.
pub async fn create_test_app() -> (Router, api::AppState, TempDir, TempDir) {
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
    let metrics_state = actors::start_metrics_actor();

    let app_state = api::AppState::new(
        csrf_config.clone(),
        session_store_for_readyz_check,
        tls_state,
        event_bus,
        network_actor,
        pkg_actor,
        metrics_state,
    );

    let shared_state = app_state.clone();
    let app = api::create_router().with_state(app_state);

    let app = auth::add_auth_routes(app)
        .layer(axum_csrf::CsrfLayer::new(csrf_config))
        .layer(session_layer)
        .layer(tower_cookies::CookieManagerLayer::new());

    (app, shared_state, temp_dir_session, temp_dir_tls)
}

/// Extract a JSON body from a response.
#[allow(dead_code)]
pub async fn get_body_as_json(response: axum::response::Response) -> Value {
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body_bytes).unwrap()
}

/// A failing session store used to exercise readiness failures.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct FailingStore;

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
