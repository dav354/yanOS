// backend/src/auth.rs

use std::{fs, path::Path, sync::Arc};

use axum::{extract::Request, middleware::Next, response::Response, routing::post, Json, Router};
use axum_csrf::CsrfConfig;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use tower_sessions::{
    cookie::Key, service::PrivateCookie, Expiry, MemoryStore, Session, SessionManagerLayer,
    SessionStore,
};
use tracing::{error, info, instrument};
use utoipa::ToSchema;

use async_trait::async_trait;
use tower_sessions_core::{
    session::{Id, Record},
    session_store,
};

use crate::error::AppError;

pub const DEFAULT_SESSION_KEY_PATH: &str = "/etc/opt/storage-os/session.key";

/// Dynamic session store wrapper so we can swap storage backends without
/// touching handler code.
#[derive(Clone, Debug)]
pub struct DynSessionStore {
    inner: Arc<dyn SessionStore + Send + Sync>,
}

impl DynSessionStore {
    pub fn new<S>(store: S) -> Self
    where
        S: SessionStore + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(store),
        }
    }
}

#[async_trait]
impl SessionStore for DynSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        self.inner.create(record).await
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        self.inner.save(record).await
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        self.inner.load(session_id).await
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        self.inner.delete(session_id).await
    }
}

/// The payload for a login request.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

/// Creates the CSRF configuration.
pub fn create_csrf_config() -> CsrfConfig {
    let mut key = [0u8; 64];
    OsRng.fill_bytes(&mut key);

    CsrfConfig::default()
        .with_key(Some(axum_csrf::Key::from(&key)))
        .with_cookie_path("/")
        .with_cookie_name("XSRF-TOKEN")
        .with_lifetime(time::Duration::hours(1))
}

fn get_or_create_session_key(path: &Path) -> Key {
    if path.exists() {
        if let Ok(content) = fs::read(path) {
            if content.len() >= 64 {
                info!("Loading session key from {:?}", path);
                return Key::from(&content);
            } else {
                info!(
                    "Existing session key file {:?} is too short ({} bytes), regenerating.",
                    path,
                    content.len()
                );
            }
        }
    }

    info!("Generating new session key at {:?}", path);
    let mut secret = [0u8; 64];
    OsRng.fill_bytes(&mut secret);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Err(e) = fs::write(path, &secret) {
        error!("Failed to write session key to {:?}: {}", path, e);
    } else {
        // Set permissions to 600
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(path, perms);
    }

    Key::from(&secret)
}

/// Creates the session management layer for the application.
pub fn create_session_layer(
    store: DynSessionStore,
) -> SessionManagerLayer<DynSessionStore, PrivateCookie> {
    let key = get_or_create_session_key(Path::new(DEFAULT_SESSION_KEY_PATH));

    SessionManagerLayer::new(store)
        .with_secure(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::hours(1)))
        .with_private(key)
}

/// Creates a test-specific session management layer for the application.
pub fn create_session_manager_layer_for_test(
    session_key_path: &Path,
) -> (
    SessionManagerLayer<DynSessionStore, PrivateCookie>,
    DynSessionStore,
) {
    let session_store = memory_store();
    let key = get_or_create_session_key(session_key_path);

    let layer = SessionManagerLayer::new(session_store.clone())
        .with_secure(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::hours(1)))
        .with_private(key);

    (layer, session_store)
}

/// Middleware to enforce authentication.
pub async fn auth_guard(session: Session, req: Request, next: Next) -> Result<Response, AppError> {
    if session
        .get::<String>("username")
        .await
        .unwrap_or(None)
        .is_some()
    {
        Ok(next.run(req).await)
    } else {
        Err(AppError::Unauthorized(
            "Authentication required".to_string(),
        ))
    }
}

/// Adds authentication-related routes to the router.
pub fn add_auth_routes<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
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

/// Simple runtime health-check against the configured session store.
pub async fn session_store_healthcheck(store: &DynSessionStore) -> Result<(), AppError> {
    let mut record = Record {
        id: Id::default(),
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + Duration::minutes(1),
    };

    store
        .create(&mut record)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Session store create failed: {e}")))?;

    store
        .delete(&record.id)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Session store delete failed: {e}")))?;

    Ok(())
}

/// Default in-memory session store wrapper.
pub fn memory_store() -> DynSessionStore {
    DynSessionStore::new(MemoryStore::default())
}
