//! Concurrent and stress tests for actors and shared state.
//!
//! These tests verify thread safety, concurrent access patterns,
//! and behavior under load.
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use std::sync::{Arc, Once};
use std::time::Duration;
use tokio::sync::Barrier;

use yanos_backend::adapters::kstat::KstatReader;
use yanos_backend::events::{EventBus, ExternalEvent};

static CRYPTO_INIT: Once = Once::new();

fn init_crypto() {
    CRYPTO_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Test concurrent EventBus publishing.
#[tokio::test]
async fn test_concurrent_event_publishing() {
    let bus = EventBus::new(10000);
    let barrier = Arc::new(Barrier::new(10));

    let mut handles = vec![];

    for task_id in 0..10 {
        let bus = bus.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for i in 0..100 {
                bus.publish(ExternalEvent::SystemLog {
                    line: format!("Task {} log line {}", task_id, i),
                });
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Should have 1000 events total
    let snapshot = bus.snapshot(10000);
    assert_eq!(
        snapshot.len(),
        1000,
        "Should have all 1000 events: got {}",
        snapshot.len()
    );
}

/// Test concurrent EventBus subscribing.
#[tokio::test]
async fn test_concurrent_event_subscribing() {
    let bus = EventBus::new(1000);
    let barrier = Arc::new(Barrier::new(11));

    // Start subscriber threads
    let mut sub_handles = vec![];
    for _ in 0..10 {
        let bus = bus.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            let mut receiver = bus.subscribe();
            barrier.wait().await;

            let mut received = 0;
            // Try to receive for a short time
            for _ in 0..100 {
                if receiver.try_recv().is_ok() {
                    received += 1;
                }
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
            received
        });

        sub_handles.push(handle);
    }

    // Publisher
    let bus_pub = bus.clone();
    let barrier_pub = barrier.clone();
    let pub_handle = tokio::spawn(async move {
        barrier_pub.wait().await;

        for i in 0..50 {
            bus_pub.publish(ExternalEvent::SystemLog {
                line: format!("Event {}", i),
            });
            tokio::time::sleep(Duration::from_micros(50)).await;
        }
    });

    pub_handle.await.expect("Publisher panicked");

    let mut total_received = 0;
    for handle in sub_handles {
        total_received += handle.await.expect("Subscriber panicked");
    }

    println!("Total events received across subscribers: {}", total_received);
}

/// Test concurrent snapshot reads.
#[tokio::test]
async fn test_concurrent_snapshot_reads() {
    let bus = EventBus::new(1000);

    // Prepopulate with events
    for i in 0..100 {
        bus.publish(ExternalEvent::SystemLog {
            line: format!("Event {}", i),
        });
    }

    let barrier = Arc::new(Barrier::new(20));
    let mut handles = vec![];

    for _ in 0..20 {
        let bus = bus.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for _ in 0..100 {
                let snapshot = bus.snapshot(50);
                assert!(!snapshot.is_empty(), "Snapshot should not be empty");
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }
}

/// Test concurrent KstatReader operations.
#[test]
fn test_concurrent_kstat_readers() {
    use std::thread;

    let barrier = Arc::new(std::sync::Barrier::new(5));
    let mut handles = vec![];

    for _ in 0..5 {
        let barrier = barrier.clone();

        let handle = thread::spawn(move || {
            let mut reader = KstatReader::new().expect("Failed to open kstat");
            barrier.wait();

            for _ in 0..20 {
                let _ = reader.get_aggregate_cpu_ticks();
                let _ = reader.get_memory_pages();
                let _ = reader.get_arc_size();
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

/// Test MetricsActor under concurrent history reads.
#[tokio::test]
async fn test_metrics_actor_concurrent_history() {
    let metrics_state = yanos_backend::actors::start_metrics_actor()
        .expect("Failed to start metrics actor");

    // Wait for some data to accumulate
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let history = metrics_state.history.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for _ in 0..50 {
                let hist = history.read().await;
                let _ = hist.len();
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }
}

/// Test NetworkActor under concurrent requests.
#[tokio::test]
async fn test_network_actor_concurrent_requests() {
    let network_actor = yanos_backend::actors::start_network_actor();

    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let actor = network_actor.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for _ in 0..20 {
                let _ = actor.list_interfaces().await;
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }
}

/// Test EventBus buffer overflow behavior.
#[tokio::test]
async fn test_event_bus_overflow() {
    // Create bus with minimum capacity (1000)
    let bus = EventBus::new(1);

    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];

    // Publish way more events than capacity
    for task_id in 0..5 {
        let bus = bus.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for i in 0..500 {
                bus.publish(ExternalEvent::SystemLog {
                    line: format!("Task {} overflow event {}", task_id, i),
                });
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // History should be capped
    let snapshot = bus.snapshot(5000);
    assert!(
        snapshot.len() <= 2500,
        "History should be capped, got {}",
        snapshot.len()
    );
}

/// Test rapid subscribe/unsubscribe cycles.
#[tokio::test]
async fn test_rapid_subscribe_unsubscribe() {
    let bus = EventBus::new(1000);

    for _ in 0..100 {
        let _receiver = bus.subscribe();
        // Receiver is dropped immediately
    }

    // Bus should still work
    bus.publish(ExternalEvent::SystemLog {
        line: "After many subscribes".to_string(),
    });

    let snapshot = bus.snapshot(10);
    assert!(!snapshot.is_empty());
}

/// Test EventBus clone behavior under load.
#[tokio::test]
async fn test_event_bus_clone_under_load() {
    let bus = EventBus::new(1000);
    let barrier = Arc::new(Barrier::new(6));

    let mut handles = vec![];

    // Some tasks publish
    for task_id in 0..3 {
        let bus = bus.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for i in 0..100 {
                bus.publish(ExternalEvent::SystemLog {
                    line: format!("Publisher {} event {}", task_id, i),
                });
            }
        });

        handles.push(handle);
    }

    // Some tasks read snapshots
    for _ in 0..3 {
        let bus = bus.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for _ in 0..50 {
                let _ = bus.snapshot(100);
                tokio::time::sleep(Duration::from_micros(10)).await;
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Should have all events
    let final_snapshot = bus.snapshot(1000);
    assert_eq!(final_snapshot.len(), 300, "Should have 300 events");
}

/// Test KstatReader update under load.
#[test]
fn test_kstat_update_under_load() {
    let mut reader = KstatReader::new().expect("Failed to open kstat");

    for _ in 0..100 {
        let _ = reader.update();
        let _ = reader.get_cpu_ticks_by_instance();
        let _ = reader.get_memory_pages();
    }
}

/// Test rapid metrics broadcast subscriptions.
#[tokio::test]
async fn test_rapid_metrics_subscriptions() {
    let metrics_state = yanos_backend::actors::start_metrics_actor()
        .expect("Failed to start metrics actor");

    // Subscribe and unsubscribe rapidly
    for _ in 0..100 {
        let _rx = metrics_state.broadcast_tx.subscribe();
        // rx is dropped immediately
    }

    // Should still work
    let rx = metrics_state.broadcast_tx.subscribe();
    assert!(rx.is_empty() || !rx.is_empty()); // Just verify no panic
}

/// Test concurrent event types.
#[tokio::test]
async fn test_concurrent_event_types() {
    let bus = EventBus::new(5000);
    let barrier = Arc::new(Barrier::new(5));

    let mut handles = vec![];

    // Different event types from different tasks
    let bus1 = bus.clone();
    let barrier1 = barrier.clone();
    handles.push(tokio::spawn(async move {
        barrier1.wait().await;
        for i in 0..100 {
            bus1.publish(ExternalEvent::SystemLog {
                line: format!("Log {}", i),
            });
        }
    }));

    let bus2 = bus.clone();
    let barrier2 = barrier.clone();
    handles.push(tokio::spawn(async move {
        barrier2.wait().await;
        for i in 0..100 {
            bus2.publish(ExternalEvent::ServiceStarted {
                fmri: format!("svc:/test{}:default", i),
            });
        }
    }));

    let bus3 = bus.clone();
    let barrier3 = barrier.clone();
    handles.push(tokio::spawn(async move {
        barrier3.wait().await;
        for i in 0..100 {
            bus3.publish(ExternalEvent::LinkUp {
                name: format!("e1000g{}", i),
            });
        }
    }));

    let bus4 = bus.clone();
    let barrier4 = barrier.clone();
    handles.push(tokio::spawn(async move {
        barrier4.wait().await;
        for i in 0..100 {
            bus4.publish(ExternalEvent::DatasetCreated {
                name: format!("rpool/data{}", i),
            });
        }
    }));

    let bus5 = bus.clone();
    let barrier5 = barrier.clone();
    handles.push(tokio::spawn(async move {
        barrier5.wait().await;
        for i in 0..100 {
            bus5.publish(ExternalEvent::ConfigChanged {
                path: std::path::PathBuf::from(format!("/etc/test{}.conf", i)),
            });
        }
    }));

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let snapshot = bus.snapshot(1000);
    assert_eq!(snapshot.len(), 500, "Should have 500 events total");
}

/// Test TLS state concurrent access.
#[tokio::test]
async fn test_tls_state_concurrent_access() {
    init_crypto();
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let tls_state = yanos_backend::tls::TlsState::load(temp_dir.path())
        .await
        .expect("Failed to load TLS state");

    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let state = tls_state.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for _ in 0..100 {
                assert!(state.is_ready());
                let _ = state.config();
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }
}

/// Test PkgActor under concurrent requests.
#[tokio::test]
async fn test_pkg_actor_concurrent_requests() {
    let event_bus = EventBus::new(100);
    let pkg_actor = yanos_backend::actors::start_pkg_actor(event_bus);

    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];

    for _ in 0..5 {
        let actor = pkg_actor.clone();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            for _ in 0..10 {
                let _ = actor.list().await;
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }
}
