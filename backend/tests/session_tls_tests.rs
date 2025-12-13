mod common;

use yanos_backend::{auth, tls};

#[tokio::test]
async fn test_session_store_healthcheck_passes_for_memory_store() {
    let store = auth::memory_store();
    auth::session_store_healthcheck(&store)
        .await
        .expect("healthcheck should pass for memory store");
}

#[tokio::test]
async fn test_session_key_is_regenerated_when_too_short() {
    let (_app, _app_state, temp_dir_session, _temp_dir_tls) = common::create_test_app().await;

    let session_key_path = temp_dir_session.path().join("session.key");
    std::fs::remove_file(&session_key_path).unwrap();

    // Write an intentionally too-short key.
    std::fs::write(&session_key_path, &[1u8]).expect("Failed to write short key");

    // Building the layer will regenerate the key.
    let (_session_layer, _session_store) =
        auth::create_session_manager_layer_for_test(&session_key_path);

    let regenerated = std::fs::read(&session_key_path).expect("Failed to read regenerated key");
    assert!(
        regenerated.len() >= 64,
        "Regenerated key should be at least 64 bytes"
    );
}

#[tokio::test]
async fn test_tls_state_load_creates_certs_and_is_ready() {
    let (_app, _app_state, _temp_dir_session, temp_dir_tls) = common::create_test_app().await;
    let tls_state = tls::TlsState::load(temp_dir_tls.path())
        .await
        .expect("Failed to load TLS state for test");

    assert!(tls_state.is_ready());
    assert!(temp_dir_tls.path().join("cert.pem").exists());
    assert!(temp_dir_tls.path().join("key.pem").exists());
}
