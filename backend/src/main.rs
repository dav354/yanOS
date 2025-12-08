use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() {
    // Define the application routes
    let app = Router::new()
        // Add the Swagger UI route
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Define the API routes under /api/v1
        .nest(
            "/api/v1",
            Router::new().route("/status", get(api_status)),
        );

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3007));
    println!("🚀 Server listening on http://{}", addr);
    println!("📚 Swagger UI available at http://{}/swagger-ui", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// API documentation definition
#[derive(OpenApi)]
#[openapi(
    paths(
        api_status,
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
struct ApiDoc;

/// Get the current status of the API
#[utoipa::path(
    get,
    path = "/api/v1/status",
    responses(
        (status = 200, description = "API is running", body = Value)
    )
)]
async fn api_status() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
