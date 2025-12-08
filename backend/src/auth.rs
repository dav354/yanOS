// backend/src/auth.rs

use axum::{routing::post, Json, Router};
use axum_csrf::{CsrfConfig, CsrfLayer};
use rand::RngCore;
use serde::Deserialize;
use time::Duration;
use tower_sessions::{
    cookie::Key, service::PrivateCookie, Expiry, MemoryStore, Session, SessionManagerLayer,
};
use tracing::{error, info, instrument};
use utoipa::ToSchema;

use crate::error::AppError;

/// The payload for a login request.
#[derive(Deserialize, ToSchema)]
pub struct LoginPayload {
    username: String,
    password: String,
}

/// Creates the CSRF configuration.
pub fn create_csrf_config() -> CsrfConfig {
    let mut key = [0u8; 32];
    rand::rng().fill_bytes(&mut key);

    CsrfConfig::default()
        .with_key(Some(axum_csrf::Key::from(&key)))
        .with_cookie_path("/")
        .with_cookie_name("XSRF-TOKEN")
        .with_lifetime(time::Duration::hours(1))
}

/// Creates the session management layer for the application.
pub fn create_session_layer() -> SessionManagerLayer<MemoryStore, PrivateCookie> {
    let session_store = MemoryStore::default();
    let mut secret = [0u8; 64];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut secret);

    SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::hours(1)))
        .with_private(Key::from(&secret))
}

/// Adds authentication-related routes to the router.
pub fn add_auth_routes(router: Router) -> Router {
    router.route("/api/login", post(login_handler))
}

/// Handles the login request, authenticating against PAM.
#[utoipa::path(
    post,
    path = "/api/login",
    request_body = LoginPayload,
    responses(
        (status = 200, description = "Authentication successful"),
        (status = 401, description = "Invalid credentials")
    )
)]
#[instrument(skip(payload, session))]
pub async fn login_handler(
    session: Session,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<&'static str>, AppError> {
    info!(username = %payload.username, "Attempting to authenticate user");

    let username = payload.username.clone();
    let password = payload.password.clone();

    // The PAM authentication needs to be run in a blocking thread
    // to avoid blocking the async runtime.
    let result = tokio::task::spawn_blocking(move || {
        let mut client = pam::Client::with_password("login")?;
        client
            .conversation_mut()
            .set_credentials(&username, &password);
        client.authenticate()
    })
    .await
    .map_err(|e| AppError::InternalServerError(format!("PAM task failed: {}", e)))?;

    match result {
        Ok(_) => {
            info!(username = %payload.username, "User authenticated successfully");
            // Store username in the session.
            session
                .insert("username", &payload.username)
                .await
                .map_err(|e| {
                    AppError::InternalServerError(format!("Failed to insert into session: {}", e))
                })?;
            Ok(Json("Authentication successful"))
        }
        Err(e) => {
            error!(username = %payload.username, "Authentication failed: {}", e);
            Err(AppError::Unauthorized("Invalid credentials".to_string()))
        }
    }
}
