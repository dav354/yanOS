use axum::extract::FromRef;
use axum_csrf::CsrfConfig;
use std::sync::Arc;

use crate::actors::{MetricsState, NetworkActorHandle, PkgActorHandle}; // Import Commands
use crate::auth::DynSessionStore;
use crate::events::EventBus;
use crate::tls::TlsState;

#[derive(Clone, Debug)]
pub struct AppState {
    pub csrf_config: CsrfConfig,
    pub session_store: DynSessionStore,
    pub tls_state: TlsState,
    pub event_bus: EventBus,
    pub network_actor: NetworkActorHandle,
    pub pkg_actor: PkgActorHandle,
    pub metrics_state: Arc<MetricsState>,
}

impl AppState {
    pub fn new(
        csrf_config: CsrfConfig,
        session_store: DynSessionStore,
        tls_state: TlsState,
        event_bus: EventBus,
        network_actor: NetworkActorHandle,
        pkg_actor: PkgActorHandle,
        metrics_state: Arc<MetricsState>,
    ) -> Self {
        Self {
            csrf_config,
            session_store,
            tls_state,
            event_bus,
            network_actor,
            pkg_actor,
            metrics_state,
        }
    }
}

impl FromRef<AppState> for CsrfConfig {
    fn from_ref(input: &AppState) -> CsrfConfig {
        input.csrf_config.clone()
    }
}

// Add FromRef for MetricsState
impl FromRef<AppState> for Arc<MetricsState> {
    fn from_ref(state: &AppState) -> Self {
        state.metrics_state.clone()
    }
}

impl FromRef<AppState> for EventBus {
    fn from_ref(state: &AppState) -> Self {
        state.event_bus.clone()
    }
}

// Ensure AppState satisfies Axum state bounds at compile time.
const _: fn() = || {
    fn assert_bounds<T: Clone + Send + Sync + 'static>() {}
    assert_bounds::<AppState>();
};
