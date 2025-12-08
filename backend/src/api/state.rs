use axum::extract::FromRef;
use axum_csrf::CsrfConfig;

use crate::actors::{NetworkActorHandle, PkgActorHandle};
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
}

impl AppState {
    pub fn new(
        csrf_config: CsrfConfig,
        session_store: DynSessionStore,
        tls_state: TlsState,
        event_bus: EventBus,
        network_actor: NetworkActorHandle,
        pkg_actor: PkgActorHandle,
    ) -> Self {
        Self {
            csrf_config,
            session_store,
            tls_state,
            event_bus,
            network_actor,
            pkg_actor,
        }
    }
}

impl FromRef<AppState> for CsrfConfig {
    fn from_ref(input: &AppState) -> CsrfConfig {
        input.csrf_config.clone()
    }
}
