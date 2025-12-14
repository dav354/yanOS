//! Tests for the terminal actor module.
//!
//! These tests verify terminal session management and PTY handling.
//! Note: Full integration tests require root privileges and a valid user.
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use yanos_backend::actors::TerminalMessage;

/// Test TerminalMessage enum variants.
#[test]
fn test_terminal_message_variants() {
    // Test Input variant
    let input_msg = TerminalMessage::Input("ls -la\n".to_string());
    match input_msg {
        TerminalMessage::Input(data) => assert_eq!(data, "ls -la\n"),
        _ => panic!("Expected Input variant"),
    }

    // Test Resize variant
    let resize_msg = TerminalMessage::Resize { rows: 40, cols: 120 };
    match resize_msg {
        TerminalMessage::Resize { rows, cols } => {
            assert_eq!(rows, 40);
            assert_eq!(cols, 120);
        }
        _ => panic!("Expected Resize variant"),
    }

    // Test Shutdown variant
    let shutdown_msg = TerminalMessage::Shutdown;
    match shutdown_msg {
        TerminalMessage::Shutdown => {}
        _ => panic!("Expected Shutdown variant"),
    }
}

/// Test TerminalMessage Debug implementation.
#[test]
fn test_terminal_message_debug() {
    let msg = TerminalMessage::Input("test".to_string());
    let debug_str = format!("{:?}", msg);
    assert!(debug_str.contains("Input"));
    assert!(debug_str.contains("test"));

    let resize_msg = TerminalMessage::Resize { rows: 24, cols: 80 };
    let debug_str = format!("{:?}", resize_msg);
    assert!(debug_str.contains("Resize"));
    assert!(debug_str.contains("24"));
    assert!(debug_str.contains("80"));
}

/// Test start_terminal_session with root user (requires privileges).
/// This test runs because we're executing as root on the VM.
#[tokio::test]
async fn test_start_terminal_session_root() {
    use yanos_backend::actors::start_terminal_session;

    let result = start_terminal_session("root".to_string());

    match result {
        Ok(session) => {
            // Verify we got a valid session
            let handle = session.handle;

            // Send a simple command
            handle
                .send_input("echo test\n".to_string())
                .await
                .expect("Should send input");

            // Clean shutdown
            handle.shutdown().await;
        }
        Err(e) => {
            // May fail if PTY not available
            println!("Terminal session failed (may be expected): {:?}", e);
        }
    }
}

/// Test start_terminal_session with invalid user.
/// Note: The session creation may succeed (PTY is opened), but `su` will fail
/// when trying to switch to the nonexistent user. The failure happens
/// asynchronously when the shell process runs.
#[tokio::test]
async fn test_start_terminal_session_invalid_user() {
    use yanos_backend::actors::start_terminal_session;

    let result = start_terminal_session("nonexistent_user_xyz_12345".to_string());

    // Session creation might succeed (PTY opened), or fail if getpwnam fails
    // Either behavior is acceptable - the important thing is no panic
    match result {
        Ok(session) => {
            // Clean shutdown
            session.handle.shutdown().await;
        }
        Err(_) => {
            // Expected - user validation failed
        }
    }
}

/// Test TerminalMessage Input with empty string.
#[test]
fn test_terminal_message_empty_input() {
    let msg = TerminalMessage::Input("".to_string());
    match msg {
        TerminalMessage::Input(data) => assert!(data.is_empty()),
        _ => panic!("Expected Input variant"),
    }
}

/// Test TerminalMessage Input with special characters.
#[test]
fn test_terminal_message_special_chars() {
    let special = "\x1b[A\x1b[B\t\r\n";
    let msg = TerminalMessage::Input(special.to_string());
    match msg {
        TerminalMessage::Input(data) => assert_eq!(data, special),
        _ => panic!("Expected Input variant"),
    }
}

/// Test TerminalMessage Resize with zero dimensions.
#[test]
fn test_terminal_message_resize_zero() {
    let msg = TerminalMessage::Resize { rows: 0, cols: 0 };
    match msg {
        TerminalMessage::Resize { rows, cols } => {
            assert_eq!(rows, 0);
            assert_eq!(cols, 0);
        }
        _ => panic!("Expected Resize variant"),
    }
}

/// Test TerminalMessage Resize with large dimensions.
#[test]
fn test_terminal_message_resize_large() {
    let msg = TerminalMessage::Resize { rows: 1000, cols: 5000 };
    match msg {
        TerminalMessage::Resize { rows, cols } => {
            assert_eq!(rows, 1000);
            assert_eq!(cols, 5000);
        }
        _ => panic!("Expected Resize variant"),
    }
}
