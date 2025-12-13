use axum::Router;
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::{Config, SwaggerUi};
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
        status::system_info,
        crate::auth::pam::login_handler,
        routes::logs::list_logs,
        routes::events::stream_events,
        routes::metrics::live_metrics,
        routes::terminal::ws_handler,
        routes::settings::get_telemetry,
        routes::settings::update_telemetry,
        resources::list_network,
        resources::list_packages,
        resources::list_updates,
        resources::check_updates,
        resources::create_dataset,
    ),
    components(
        schemas(
            status::StatusResponse,
            crate::auth::LoginPayload,
            crate::events::LoggedEvent,
            crate::events::ExternalEvent,
            resources::CreateDatasetRequest,
            resources::CreateDatasetResponse,
            routes::settings::TelemetrySettings,
            // crate::error::ErrorResponse // If this doesn't exist, we should remove it or define it.
            // Using generic error for now or checking if we can use a local struct.
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "yanos", description = "yanOS Management API")
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "basic_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Basic)),
        );
    }
}

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
                .config(
                    Config::from("/api-docs/openapi.json")
                        .persist_authorization(true)
                        .try_it_out_enabled(true),
                ),
        )
        .nest("/api/v1", routes::mod_routes())
}
