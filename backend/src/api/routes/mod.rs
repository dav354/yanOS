use crate::api::AppState;
use crate::auth;
use axum::routing::get;
use axum::{Router, middleware};

pub mod events;
pub mod health;
pub mod logs;
pub mod metrics;
pub mod resources;
pub mod status;
pub mod terminal; // Add terminal module
pub mod settings;

// Re-export handlers needed for Utoipa documentation in api/mod.rs
pub use health::{healthz, readyz};
pub use resources::{create_dataset, list_network, list_packages};
pub use status::{api_status, get_status, system_info};

pub fn mod_routes() -> Router<AppState> {
    let protected = Router::<AppState>::new()
        .merge(resources::routes())
        .route("/events", get(events::stream_events))
        .route("/metrics/live", get(metrics::live_metrics))
        .route("/logs", get(logs::list_logs))
        .route("/terminal", get(terminal::ws_handler)) // Add terminal route
        .merge(settings::routes())
        .route_layer(middleware::from_fn(auth::auth_guard));

    Router::<AppState>::new()
        .merge(health::routes())
        .merge(status::routes())
        .merge(protected)
}
