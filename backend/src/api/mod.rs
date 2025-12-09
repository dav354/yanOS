use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
// Route handlers need to be public for utoipa macro to see them
pub use crate::api::routes::{health, resources, status};

pub mod routes;
pub mod state;

// Re-export AppState
pub use state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        health::healthz,
        health::readyz,
        status::get_status,
        resources::create_dataset,
    ),
    components(
        schemas(
            status::StatusResponse,
            resources::CreateDatasetRequest,
            resources::CreateDatasetResponse,
            // crate::error::ErrorResponse // If this doesn't exist, we should remove it or define it.
            // Using generic error for now or checking if we can use a local struct.
        )
    ),
    tags(
        (name = "zos", description = "zOS Management API")
    )
)]
struct ApiDoc;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", routes::mod_routes())
}
