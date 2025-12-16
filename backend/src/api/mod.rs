use axum::Router;
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::{Config, SwaggerUi};
// Route handlers need to be public for utoipa macro to see them
pub use crate::api::routes::{health, network, resources, status};

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
        crate::auth::pam::logout_handler,
        routes::logs::list_logs,
        routes::events::stream_events,
        routes::metrics::live_metrics,
        routes::terminal::ws_handler,
        routes::settings::get_telemetry,
        routes::settings::update_telemetry,
        // Network routes
        network::list_interfaces,
        network::list_links,
        network::get_config,
        network::update_config,
        network::set_address,
        network::set_dhcp,
        // Package routes
        resources::list_packages,
        resources::list_updates,
        resources::check_updates,
        resources::create_dataset,
        // Storage routes
        routes::storage::list_pools,
        routes::storage::get_pool,
        routes::storage::list_datasets,
        routes::storage::get_dataset,
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
            crate::adapters::zfs::PoolInfo,
            crate::adapters::zfs::DatasetInfo,
            // Network schemas
            crate::core::NetworkInterface,
            crate::core::PhysicalLink,
            crate::core::NetworkConfig,
            network::SetAddressRequest,
            network::UpdateConfigRequest,
            network::SuccessResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "yanos", description = "yanOS Management API"),
        (name = "storage", description = "ZFS Storage Management"),
        (name = "network", description = "Network Configuration")
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
