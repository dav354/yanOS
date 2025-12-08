pub mod routes;
pub mod state;

use axum::{Router, middleware, routing::get};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::auth;
pub use state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::status::api_status,
        routes::health::healthz_handler,
        routes::health::readyz_handler,
        crate::auth::pam::login_handler,
        routes::status::system_info,
        routes::resources::list_network,
        routes::resources::list_packages,
    ),
    components(
        schemas(auth::LoginPayload)
    ),
    tags(
        (name = "zOS", description = "zOS Management API")
    ),
    info(
        title = "zOS API",
        version = "1.0.0",
        description = "API for managing the zOS Storage Appliance",
    )
)]
pub struct ApiDoc;

/// Creates the main API router, including the Swagger UI.
pub fn create_router(app_state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/events", axum::routing::get(routes::stream_events))
        .route("/metrics/live", axum::routing::get(routes::stream_metrics))
        .route_layer(middleware::from_fn(auth::auth_guard));

    let api_routes = Router::new()
        .route("/status", get(routes::api_status))
        .route("/system/info", get(routes::system_info))
        .route("/network/interfaces", get(routes::list_network))
        .route("/pkg/list", get(routes::list_packages))
        .merge(protected_routes);

    Router::new()
        .route("/healthz", get(routes::healthz_handler))
        .route("/readyz", get(routes::readyz_handler))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", api_routes)
        .with_state(app_state)
}
