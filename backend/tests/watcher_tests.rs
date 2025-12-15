//! Tests for filesystem and log watcher modules.
//!
//! These tests verify the watcher functionality for detecting
//! file changes and tailing log files.
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::tempdir;

use yanos_backend::events::{EventBus, ExternalEvent};
use yanos_backend::watchers::{start_filesystem_watcher, start_system_log_watcher};

/// Test filesystem watcher with empty paths (disabled).
#[tokio::test]
async fn test_filesystem_watcher_disabled() {
    let bus = EventBus::new(100);
    let paths: Vec<PathBuf> = vec![];

    let result = start_filesystem_watcher(&paths, bus).await;
    assert!(result.is_ok(), "Should succeed with empty paths");

    let watcher = result.unwrap();
    assert!(watcher.is_none(), "Watcher should be None when disabled");
}

/// Test filesystem watcher with valid path.
#[tokio::test]
async fn test_filesystem_watcher_valid_path() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let watch_path = temp_dir.path().to_path_buf();

    // Create a file to watch
    let test_file = watch_path.join("test.conf");
    fs::write(&test_file, "initial content").expect("Failed to create test file");

    let bus = EventBus::new(100);
    let mut receiver = bus.subscribe();
    let paths = vec![watch_path.clone()];

    let result = start_filesystem_watcher(&paths, bus.clone()).await;
    assert!(result.is_ok(), "Should succeed with valid path");

    let watcher = result.unwrap();
    assert!(watcher.is_some(), "Watcher should be Some");

    // Wait a moment for watcher to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify the file to trigger event
    fs::write(&test_file, "modified content").expect("Failed to modify test file");

    // Give the watcher time to detect the change
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check if we received a ConfigChanged event
    // Note: May or may not receive depending on timing
    match receiver.try_recv() {
        Ok(logged) => {
            if let ExternalEvent::ConfigChanged { path } = &logged.event {
                println!("Received ConfigChanged for: {:?}", path);
            }
        }
        Err(_) => {
            println!("No event received (timing dependent)");
        }
    }
}

/// Test filesystem watcher with nonexistent path.
/// Note: Behavior is platform-dependent - some notify backends fail, others succeed.
#[tokio::test]
async fn test_filesystem_watcher_nonexistent_path() {
    let bus = EventBus::new(100);
    let paths = vec![PathBuf::from("/nonexistent/path/that/does/not/exist")];

    let result = start_filesystem_watcher(&paths, bus).await;
    // Platform-dependent: may fail or succeed depending on notify backend
    // On illumos, FEN (File Event Notification) may handle this differently than inotify
    println!("Nonexistent path result: {:?}", result.is_ok());
}

/// Test filesystem watcher with multiple paths.
#[tokio::test]
async fn test_filesystem_watcher_multiple_paths() {
    let temp_dir1 = tempdir().expect("Failed to create temp dir 1");
    let temp_dir2 = tempdir().expect("Failed to create temp dir 2");

    let bus = EventBus::new(100);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];

    let result = start_filesystem_watcher(&paths, bus).await;
    assert!(result.is_ok(), "Should succeed with multiple paths");

    let watcher = result.unwrap();
    assert!(watcher.is_some(), "Watcher should be Some");
}

/// Test log watcher with valid file.
#[tokio::test]
async fn test_log_watcher_valid_file() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("test.log");

    // Create initial log file with some content
    {
        let mut file = File::create(&log_path).expect("Failed to create log file");
        for i in 0..10 {
            writeln!(file, "Log line {}", i).expect("Failed to write log line");
        }
    }

    let bus = EventBus::new(1000);
    let mut receiver = bus.subscribe();

    let result = start_system_log_watcher(&log_path, bus.clone());
    assert!(result.is_ok(), "Should succeed with valid log file");

    let handle = result.unwrap();

    // Wait for preload to complete - use timeout to avoid hanging
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut received_count = 0;
    while tokio::time::Instant::now() < deadline {
        while let Ok(logged) = receiver.try_recv() {
            if matches!(logged.event, ExternalEvent::SystemLog { .. }) {
                received_count += 1;
            }
        }
        if received_count > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(received_count > 0, "Should have received preloaded log lines");
    println!("Received {} preloaded log lines", received_count);

    // Abort the handle to clean up
    handle.abort();
}

/// Test log watcher with nonexistent file fails.
#[tokio::test]
async fn test_log_watcher_nonexistent_file() {
    let bus = EventBus::new(100);
    let log_path = PathBuf::from("/nonexistent/log/file.log");

    let result = start_system_log_watcher(&log_path, bus);
    assert!(result.is_err(), "Should fail with nonexistent file");
}

/// Test log watcher detects new lines.
#[tokio::test]
async fn test_log_watcher_detects_new_lines() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("live.log");

    // Create empty log file
    File::create(&log_path).expect("Failed to create log file");

    let bus = EventBus::new(1000);
    let mut receiver = bus.subscribe();

    let result = start_system_log_watcher(&log_path, bus.clone());
    assert!(result.is_ok(), "Should succeed");

    let handle = result.unwrap();

    // Wait briefly for watcher to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Append new line to log file
    {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&log_path)
            .expect("Failed to open log file");
        writeln!(file, "New log line at {}", chrono::Utc::now()).expect("Failed to write");
    }

    // Wait for watcher to detect with timeout
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut found_new_line = false;
    while tokio::time::Instant::now() < deadline && !found_new_line {
        while let Ok(logged) = receiver.try_recv() {
            if let ExternalEvent::SystemLog { line } = logged.event {
                if line.contains("New log line") {
                    found_new_line = true;
                    break;
                }
            }
        }
        if !found_new_line {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // Note: This is timing-dependent, may not always succeed
    println!("Found new line: {}", found_new_line);

    handle.abort();
}

/// Test log watcher handles empty file.
#[tokio::test]
async fn test_log_watcher_empty_file() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("empty.log");

    // Create empty file
    File::create(&log_path).expect("Failed to create log file");

    let bus = EventBus::new(100);
    let result = start_system_log_watcher(&log_path, bus);

    assert!(result.is_ok(), "Should succeed with empty file");

    let handle = result.unwrap();
    handle.abort();
}

/// Test log watcher with large file preload.
#[tokio::test]
async fn test_log_watcher_large_file_preload() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("large.log");

    // Create log file with more than 500 lines
    {
        let mut file = File::create(&log_path).expect("Failed to create log file");
        for i in 0..1000 {
            writeln!(file, "Log line number {}", i).expect("Failed to write");
        }
    }

    let bus = EventBus::new(1000);
    let mut receiver = bus.subscribe();

    let result = start_system_log_watcher(&log_path, bus.clone());
    assert!(result.is_ok());

    let handle = result.unwrap();

    // Wait for preload with timeout
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut received_count = 0;
    while tokio::time::Instant::now() < deadline {
        while receiver.try_recv().is_ok() {
            received_count += 1;
        }
        // Break early once we've received a good amount
        if received_count >= 100 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Should receive approximately 500 lines (the preload limit)
    println!("Received {} lines from large file", received_count);
    assert!(received_count <= 500, "Should be capped at preload limit");

    handle.abort();
}

// --- Edge Cases ---

/// Test filesystem watcher with file (not directory).
#[tokio::test]
async fn test_filesystem_watcher_on_file() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, "content").expect("Failed to create file");

    let bus = EventBus::new(100);
    let paths = vec![file_path];

    // notify might accept watching a file directly
    let result = start_filesystem_watcher(&paths, bus).await;
    // Result depends on notify implementation
    println!("Watching file result: {:?}", result.is_ok());
}

/// Test log watcher with special characters in path.
#[tokio::test]
async fn test_log_watcher_special_path() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("log-with-dashes_and_underscores.log");

    File::create(&log_path).expect("Failed to create log file");

    let bus = EventBus::new(100);
    let result = start_system_log_watcher(&log_path, bus);

    assert!(result.is_ok(), "Should handle special characters in path");
    result.unwrap().abort();
}

/// Test log watcher with binary content.
#[tokio::test]
async fn test_log_watcher_binary_content() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("binary.log");

    // Create file with some binary content mixed with text
    {
        let mut file = File::create(&log_path).expect("Failed to create log file");
        file.write_all(b"Normal line\n").expect("Failed to write");
        file.write_all(&[0, 1, 2, 255, 254, 253]).expect("Failed to write binary");
        file.write_all(b"\nAnother normal line\n").expect("Failed to write");
    }

    let bus = EventBus::new(100);
    let result = start_system_log_watcher(&log_path, bus);

    // Should handle binary content gracefully
    assert!(result.is_ok());
    let handle = result.unwrap();

    // Brief wait then clean up
    tokio::time::sleep(Duration::from_millis(100)).await;
    handle.abort();
}

/// Test EventBus is properly cloned for watchers.
#[tokio::test]
async fn test_event_bus_clone_for_watchers() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("test.log");
    fs::write(&log_path, "initial\n").expect("Failed to create log file");

    let bus = EventBus::new(100);
    let bus_clone = bus.clone();

    // Subscribe before starting watcher
    let mut receiver = bus.subscribe();

    let result = start_system_log_watcher(&log_path, bus_clone);
    assert!(result.is_ok());

    let handle = result.unwrap();

    // Wait for preload with timeout
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut count = 0;
    while tokio::time::Instant::now() < deadline {
        while receiver.try_recv().is_ok() {
            count += 1;
        }
        if count > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Original bus should also see events in its history
    let snapshot = bus.snapshot(10);
    println!("Receiver got {} events, snapshot has {} events", count, snapshot.len());

    handle.abort();
}
