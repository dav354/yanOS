use axum_csrf::CsrfConfig;
use rand::{RngCore, thread_rng};

/// Creates the CSRF configuration.
pub fn create_csrf_config() -> CsrfConfig {
    let mut key = [0u8; 64];
    let mut rng_instance = thread_rng();
    rng_instance.fill_bytes(&mut key);

    CsrfConfig::default()
        .with_key(Some(axum_csrf::Key::from(&key)))
        .with_cookie_path("/")
        .with_cookie_name("XSRF-TOKEN")
        .with_lifetime(time::Duration::hours(1))
}
