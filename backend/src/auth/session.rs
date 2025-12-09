use std::{fs, path::Path, sync::Arc};

use time::{Duration, OffsetDateTime};
use tower_sessions::{
    Expiry, MemoryStore, SessionManagerLayer, SessionStore, cookie::Key, service::PrivateCookie,
};
use tracing::{error, info};

use async_trait::async_trait;
use tower_sessions_core::{
    session::{Id, Record},
    session_store,
};

use crate::error::AppError;

pub const DEFAULT_SESSION_KEY_PATH: &str = "/etc/opt/zos/session.key";

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
    let mut rng_instance = rand::thread_rng();
    rand::RngCore::fill_bytes(&mut rng_instance, &mut secret);

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
