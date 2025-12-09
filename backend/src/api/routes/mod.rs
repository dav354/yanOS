use crate::api::AppState;
use crate::auth;
use axum::routing::get;
use axum::{Router, middleware};

pub mod events;
pub mod health;
pub mod metrics;
pub mod resources;
pub mod status;

// Re-export handlers needed for Utoipa documentation in api/mod.rs
pub use health::{healthz, readyz};
pub use resources::{create_dataset, list_network, list_packages};
pub use status::{api_status, get_status, system_info};
// Events are websocket, maybe no need to export handler for openapi if not documented there

pub fn mod_routes() -> Router<AppState> {
    let protected = Router::<AppState>::new()
        .merge(resources::routes())
        .route("/events", get(events::stream_events))
        .route("/metrics/live", get(metrics::live_metrics))
        .route_layer(middleware::from_fn(auth::auth_guard));

    Router::<AppState>::new()
        .merge(health::routes())
        .merge(status::routes())
        .merge(protected)
}
