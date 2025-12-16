use crate::api::AppState;
use crate::auth;
use axum::middleware;
use axum::routing::get;
use axum::Router;

pub mod events;
pub mod health;
pub mod logs;
pub mod metrics;
pub mod network;
pub mod resources;
pub mod settings;
pub mod status;
pub mod storage;
pub mod terminal;

// Re-export handlers needed for Utoipa documentation in api/mod.rs
pub use health::{healthz, readyz};
pub use network::{get_config, list_interfaces, list_links, set_address, set_dhcp, update_config};
pub use resources::{create_dataset, list_packages};
pub use status::{api_status, get_status, system_info};
pub use storage::{get_dataset, get_pool, list_datasets, list_pools};

pub fn mod_routes() -> Router<AppState> {
    let protected = Router::<AppState>::new()
        .merge(resources::routes())
        .merge(storage::routes())
        .merge(network::routes())
        .route("/events", get(events::stream_events))
        .route("/metrics/live", get(metrics::live_metrics))
        .route("/logs", get(logs::list_logs))
        .route("/terminal", get(terminal::ws_handler))
        .merge(settings::routes())
        .route_layer(middleware::from_fn(auth::auth_guard));

    Router::<AppState>::new()
        .merge(health::routes())
        .merge(status::routes())
        .merge(protected)
}
