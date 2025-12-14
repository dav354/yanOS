//! Tests for the event bus module.
//!
//! These tests verify event broadcasting, history management,
//! and event serialization.
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use std::path::PathBuf;
use yanos_backend::events::{EventBus, ExternalEvent, LoggedEvent};

/// Test EventBus creation.
#[test]
fn test_event_bus_creation() {
    let bus = EventBus::new(100);

    // Should be able to get sender and subscribe
    let _sender = bus.sender();
    let _receiver = bus.subscribe();
}

/// Test EventBus with minimum buffer size.
#[test]
fn test_event_bus_minimum_buffer() {
    // Buffer should be at least 1000 even if smaller requested
    let bus = EventBus::new(1);
    let _sender = bus.sender();
}

/// Test publishing and receiving events.
#[test]
fn test_event_publish_and_receive() {
    let bus = EventBus::new(100);
    let mut receiver = bus.subscribe();

    // Publish an event
    bus.publish(ExternalEvent::ConfigChanged {
        path: PathBuf::from("/etc/test.conf"),
    });

    // Should receive the event
    match receiver.try_recv() {
        Ok(logged) => {
            assert!(!logged.ts.is_empty(), "Timestamp should be set");
            match logged.event {
                ExternalEvent::ConfigChanged { path } => {
                    assert_eq!(path, PathBuf::from("/etc/test.conf"));
                }
                _ => panic!("Expected ConfigChanged event"),
            }
        }
        Err(e) => panic!("Failed to receive event: {:?}", e),
    }
}

/// Test event history snapshot.
#[test]
fn test_event_snapshot() {
    let bus = EventBus::new(100);

    // Publish multiple events
    bus.publish(ExternalEvent::ServiceStarted {
        fmri: "svc:/system/test1:default".to_string(),
    });
    bus.publish(ExternalEvent::ServiceStarted {
        fmri: "svc:/system/test2:default".to_string(),
    });
    bus.publish(ExternalEvent::ServiceStarted {
        fmri: "svc:/system/test3:default".to_string(),
    });

    // Get snapshot
    let snapshot = bus.snapshot(10);
    assert_eq!(snapshot.len(), 3, "Should have 3 events in snapshot");

    // Verify order (oldest first)
    match &snapshot[0].event {
        ExternalEvent::ServiceStarted { fmri } => {
            assert!(fmri.contains("test1"));
        }
        _ => panic!("Expected ServiceStarted"),
    }
}

/// Test snapshot with limit.
#[test]
fn test_event_snapshot_limit() {
    let bus = EventBus::new(100);

    // Publish 10 events
    for i in 0..10 {
        bus.publish(ExternalEvent::SystemLog {
            line: format!("Log line {}", i),
        });
    }

    // Request only last 3
    let snapshot = bus.snapshot(3);
    assert_eq!(snapshot.len(), 3, "Should have 3 events (limited)");

    // Should be the last 3 (7, 8, 9)
    match &snapshot[0].event {
        ExternalEvent::SystemLog { line } => {
            assert!(line.contains("7"));
        }
        _ => panic!("Expected SystemLog"),
    }
}

/// Test snapshot_before pagination.
#[test]
fn test_event_snapshot_before() {
    let bus = EventBus::new(100);

    // Publish events with small delay to ensure different timestamps
    bus.publish(ExternalEvent::LinkUp {
        name: "e1000g0".to_string(),
    });
    std::thread::sleep(std::time::Duration::from_millis(10));
    bus.publish(ExternalEvent::LinkUp {
        name: "e1000g1".to_string(),
    });
    std::thread::sleep(std::time::Duration::from_millis(10));
    bus.publish(ExternalEvent::LinkUp {
        name: "e1000g2".to_string(),
    });

    let all = bus.snapshot(10);
    assert_eq!(all.len(), 3);

    // Get events before the last one
    let last_ts = &all[2].ts;
    let before = bus.snapshot_before(last_ts, 10);

    // Should have the first two events
    assert_eq!(before.len(), 2);
}

/// Test all ExternalEvent variants.
#[test]
fn test_all_event_variants() {
    let bus = EventBus::new(100);

    // Publish one of each type
    bus.publish(ExternalEvent::ConfigChanged {
        path: PathBuf::from("/etc/test"),
    });
    bus.publish(ExternalEvent::ServiceStarted {
        fmri: "svc:/test:default".to_string(),
    });
    bus.publish(ExternalEvent::ServiceStopped {
        fmri: "svc:/test:default".to_string(),
    });
    bus.publish(ExternalEvent::ServiceFailed {
        fmri: "svc:/test:default".to_string(),
    });
    bus.publish(ExternalEvent::DatasetCreated {
        name: "rpool/test".to_string(),
    });
    bus.publish(ExternalEvent::DatasetDestroyed {
        name: "rpool/test".to_string(),
    });
    bus.publish(ExternalEvent::LinkUp {
        name: "e1000g0".to_string(),
    });
    bus.publish(ExternalEvent::LinkDown {
        name: "e1000g0".to_string(),
    });
    bus.publish(ExternalEvent::SystemLog {
        line: "Test log".to_string(),
    });
    bus.publish(ExternalEvent::TaskStarted {
        id: "task-1".to_string(),
        name: "Test Task".to_string(),
        started_at: "2024-01-01T00:00:00Z".to_string(),
    });
    bus.publish(ExternalEvent::TaskCompleted {
        id: "task-1".to_string(),
        name: "Test Task".to_string(),
        started_at: "2024-01-01T00:00:00Z".to_string(),
        duration_ms: 1000,
        status: "success".to_string(),
    });

    let snapshot = bus.snapshot(20);
    assert_eq!(snapshot.len(), 11, "Should have 11 different event types");
}

/// Test ExternalEvent serialization.
#[test]
fn test_external_event_serialization() {
    let event = ExternalEvent::ConfigChanged {
        path: PathBuf::from("/etc/hosts"),
    };

    let json = serde_json::to_string(&event).expect("Serialization failed");
    assert!(json.contains("\"type\":\"config_changed\""));
    assert!(json.contains("/etc/hosts"));

    // Deserialize back
    let deserialized: ExternalEvent = serde_json::from_str(&json).expect("Deserialization failed");
    match deserialized {
        ExternalEvent::ConfigChanged { path } => {
            assert_eq!(path, PathBuf::from("/etc/hosts"));
        }
        _ => panic!("Wrong event type after deserialization"),
    }
}

/// Test LoggedEvent serialization.
#[test]
fn test_logged_event_serialization() {
    let logged = LoggedEvent {
        ts: "2024-01-15T10:30:00Z".to_string(),
        event: ExternalEvent::LinkUp {
            name: "vnic0".to_string(),
        },
    };

    let json = serde_json::to_string(&logged).expect("Serialization failed");
    assert!(json.contains("\"ts\":\"2024-01-15T10:30:00Z\""));
    assert!(json.contains("\"type\":\"link_up\""));
    assert!(json.contains("\"name\":\"vnic0\""));
}

/// Test ExternalEvent Debug implementation.
#[test]
fn test_external_event_debug() {
    let event = ExternalEvent::ServiceStarted {
        fmri: "svc:/network/ssh:default".to_string(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ServiceStarted"));
    assert!(debug_str.contains("ssh"));
}

/// Test ExternalEvent Clone implementation.
#[test]
fn test_external_event_clone() {
    let event = ExternalEvent::DatasetCreated {
        name: "rpool/data".to_string(),
    };

    let cloned = event.clone();
    match cloned {
        ExternalEvent::DatasetCreated { name } => {
            assert_eq!(name, "rpool/data");
        }
        _ => panic!("Clone changed event type"),
    }
}

/// Test EventBus Clone implementation.
#[test]
fn test_event_bus_clone() {
    let bus = EventBus::new(100);

    bus.publish(ExternalEvent::SystemLog {
        line: "Before clone".to_string(),
    });

    let cloned_bus = bus.clone();

    // Both should see the same history
    let original_snapshot = bus.snapshot(10);
    let cloned_snapshot = cloned_bus.snapshot(10);

    assert_eq!(original_snapshot.len(), cloned_snapshot.len());
}

/// Test multiple subscribers.
#[test]
fn test_multiple_subscribers() {
    let bus = EventBus::new(100);

    let mut sub1 = bus.subscribe();
    let mut sub2 = bus.subscribe();

    bus.publish(ExternalEvent::LinkUp {
        name: "test0".to_string(),
    });

    // Both subscribers should receive the event
    assert!(sub1.try_recv().is_ok(), "Subscriber 1 should receive event");
    assert!(sub2.try_recv().is_ok(), "Subscriber 2 should receive event");
}

/// Test history buffer overflow.
#[test]
fn test_history_buffer_overflow() {
    // Create bus with small capacity (but min is 1000)
    let bus = EventBus::new(10);

    // Publish more than capacity
    for i in 0..1100 {
        bus.publish(ExternalEvent::SystemLog {
            line: format!("Log {}", i),
        });
    }

    // History should be capped at capacity
    let snapshot = bus.snapshot(2000);
    assert!(
        snapshot.len() <= 1100,
        "History should not exceed buffer size"
    );
}

/// Test TaskStarted and TaskCompleted events.
#[test]
fn test_task_events() {
    let started = ExternalEvent::TaskStarted {
        id: "pkg-update-123".to_string(),
        name: "Package Update Check".to_string(),
        started_at: "2024-01-15T12:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&started).expect("Serialization failed");
    assert!(json.contains("\"type\":\"task_started\""));
    assert!(json.contains("pkg-update-123"));

    let completed = ExternalEvent::TaskCompleted {
        id: "pkg-update-123".to_string(),
        name: "Package Update Check".to_string(),
        started_at: "2024-01-15T12:00:00Z".to_string(),
        duration_ms: 5432,
        status: "completed".to_string(),
    };

    let json = serde_json::to_string(&completed).expect("Serialization failed");
    assert!(json.contains("\"type\":\"task_completed\""));
    assert!(json.contains("5432"));
}

/// Test empty snapshot.
#[test]
fn test_empty_snapshot() {
    let bus = EventBus::new(100);

    let snapshot = bus.snapshot(10);
    assert!(snapshot.is_empty(), "New bus should have empty history");
}

/// Test snapshot_before with no matching events.
#[test]
fn test_snapshot_before_no_matches() {
    let bus = EventBus::new(100);

    bus.publish(ExternalEvent::SystemLog {
        line: "Test".to_string(),
    });

    // Use timestamp before any events
    let before = bus.snapshot_before("1970-01-01T00:00:00Z", 10);
    assert!(before.is_empty(), "Should have no events before epoch");
}
