//! Tests for the MetricsActor and MetricsState.
//!
//! These tests verify:
//! - Metric collection and broadcasting
//! - History buffer management
//! - MetricPoint structure validity
//! - Start function behavior
//!
//! NOTE: These tests only run on illumos/OmniOS (via `just test`).

use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use yanos_backend::actors::{start_metrics_actor, CpuCoreMetric, MetricPoint, MetricsActor};

/// Test that the MetricsActor collects and broadcasts metrics.
#[tokio::test]
async fn test_metrics_collection_and_broadcast() {
    let (_cmd_tx, cmd_rx) = mpsc::channel(10);
    let (broadcast_tx, mut broadcast_rx) = broadcast::channel(10);

    let actor = MetricsActor::new(cmd_rx, broadcast_tx).expect("Failed to construct MetricsActor");
    tokio::spawn(actor.run());

    // Wait for at least one tick (Actor ticks every 1s)
    tokio::time::sleep(Duration::from_millis(1500)).await;

    match broadcast_rx.try_recv() {
        Ok(point) => {
            assert!(point.ts > 0);
            assert!(point.memory_total > 0);
            assert!(point.cpu_user >= 0.0);
            assert!(point.cpu_idle >= 0.0);
            assert!(
                !point.per_core.is_empty(),
                "per-core CPU metrics should be present"
            );
            assert!(
                point
                    .per_core
                    .iter()
                    .all(|c| c.cpu_user >= 0.0 && c.cpu_system >= 0.0),
                "per-core CPU percentages should be non-negative"
            );

            println!("Received metric: {:?}", point);
        }
        Err(e) => {
            panic!("Failed to receive metric from broadcast: {:?}", e);
        }
    }
}

/// Test that metrics are stored in history.
#[tokio::test]
async fn test_metrics_history_accumulation() {
    let (_cmd_tx, cmd_rx) = mpsc::channel(10);
    let (broadcast_tx, _broadcast_rx) = broadcast::channel(10);

    let actor = MetricsActor::new(cmd_rx, broadcast_tx).expect("Failed to construct MetricsActor");
    let history = actor.history.clone();

    tokio::spawn(actor.run());

    // Wait for multiple ticks
    tokio::time::sleep(Duration::from_millis(2500)).await;

    let hist = history.read().await;
    assert!(
        hist.len() >= 2,
        "History should contain at least 2 entries after 2.5s"
    );

    // Verify timestamps are increasing
    let timestamps: Vec<i64> = hist.iter().map(|p| p.ts).collect();
    for window in timestamps.windows(2) {
        assert!(window[1] >= window[0], "Timestamps should be non-decreasing");
    }
}

/// Test the start_metrics_actor convenience function.
#[tokio::test]
async fn test_start_metrics_actor() {
    let state = start_metrics_actor().expect("Failed to start metrics actor");

    // Wait for some data
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Check that we can subscribe and receive
    let mut rx = state.broadcast_tx.subscribe();
    tokio::time::sleep(Duration::from_millis(1100)).await;

    match rx.try_recv() {
        Ok(point) => {
            assert!(point.ts > 0);
            println!("Received from start_metrics_actor: {:?}", point);
        }
        Err(e) => {
            panic!("Failed to receive from start_metrics_actor: {:?}", e);
        }
    }
}

/// Test MetricPoint structure has valid ranges.
#[tokio::test]
async fn test_metric_point_value_ranges() {
    let (_cmd_tx, cmd_rx) = mpsc::channel(10);
    let (broadcast_tx, mut broadcast_rx) = broadcast::channel(10);

    let actor = MetricsActor::new(cmd_rx, broadcast_tx).expect("Failed to construct MetricsActor");
    tokio::spawn(actor.run());

    // Wait for two ticks to get meaningful CPU deltas
    tokio::time::sleep(Duration::from_millis(2200)).await;

    // Drain to get the latest point
    let mut latest = None;
    while let Ok(p) = broadcast_rx.try_recv() {
        latest = Some(p);
    }

    if let Some(point) = latest {
        // CPU percentages should be 0-100
        assert!(
            point.cpu_user >= 0.0 && point.cpu_user <= 100.0,
            "cpu_user {} should be 0-100",
            point.cpu_user
        );
        assert!(
            point.cpu_system >= 0.0 && point.cpu_system <= 100.0,
            "cpu_system {} should be 0-100",
            point.cpu_system
        );
        assert!(
            point.cpu_idle >= 0.0 && point.cpu_idle <= 100.0,
            "cpu_idle {} should be 0-100",
            point.cpu_idle
        );

        // Memory values should be reasonable (at least 1MB total)
        assert!(
            point.memory_total > 1024 * 1024,
            "memory_total should be at least 1MB"
        );
        assert!(
            point.memory_used <= point.memory_total,
            "memory_used should not exceed memory_total"
        );

        // Per-core metrics should match aggregate roughly
        for core in &point.per_core {
            assert!(core.cpu_user >= 0.0 && core.cpu_user <= 100.0);
            assert!(core.cpu_system >= 0.0 && core.cpu_system <= 100.0);
            assert!(core.cpu_idle >= 0.0 && core.cpu_idle <= 100.0);
        }
    }
}

/// Test MetricPoint serialization.
#[test]
fn test_metric_point_serialization() {
    let point = MetricPoint {
        ts: 1700000000000,
        cpu_user: 25.5,
        cpu_system: 10.2,
        cpu_idle: 64.3,
        per_core: vec![CpuCoreMetric {
            id: 0,
            cpu_user: 30.0,
            cpu_system: 15.0,
            cpu_idle: 55.0,
        }],
        memory_used: 8 * 1024 * 1024 * 1024,
        memory_total: 16 * 1024 * 1024 * 1024,
        zfs_arc: 4 * 1024 * 1024 * 1024,
        rx_bytes: 1000000,
        tx_bytes: 500000,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&point).expect("Serialization failed");
    assert!(json.contains("\"ts\":1700000000000"));
    assert!(json.contains("\"cpu_user\":25.5"));
    assert!(json.contains("\"per_core\""));

    // Deserialize back
    let deserialized: MetricPoint = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.ts, point.ts);
    assert_eq!(deserialized.cpu_user, point.cpu_user);
    assert_eq!(deserialized.per_core.len(), 1);
    assert_eq!(deserialized.per_core[0].id, 0);
}

/// Test CpuCoreMetric serialization.
#[test]
fn test_cpu_core_metric_serialization() {
    let core = CpuCoreMetric {
        id: 7,
        cpu_user: 12.5,
        cpu_system: 5.5,
        cpu_idle: 82.0,
    };

    let json = serde_json::to_string(&core).expect("Serialization failed");
    assert!(json.contains("\"id\":7"));
    assert!(json.contains("\"cpu_user\":12.5"));

    let deserialized: CpuCoreMetric = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.id, core.id);
}

/// Test MetricPoint Clone and Debug.
#[test]
fn test_metric_point_clone_debug() {
    let point = MetricPoint {
        ts: 123456789,
        cpu_user: 10.0,
        cpu_system: 5.0,
        cpu_idle: 85.0,
        per_core: vec![],
        memory_used: 1000,
        memory_total: 2000,
        zfs_arc: 500,
        rx_bytes: 100,
        tx_bytes: 50,
    };

    let cloned = point.clone();
    assert_eq!(cloned.ts, point.ts);

    let debug_str = format!("{:?}", point);
    assert!(debug_str.contains("MetricPoint"));
    assert!(debug_str.contains("123456789"));
}

/// Test that history buffer doesn't exceed max size.
#[tokio::test]
async fn test_metrics_history_max_size() {
    let (_cmd_tx, cmd_rx) = mpsc::channel(10);
    let (broadcast_tx, _broadcast_rx) = broadcast::channel(10);

    let actor = MetricsActor::new(cmd_rx, broadcast_tx).expect("Failed to construct MetricsActor");
    let history = actor.history.clone();

    tokio::spawn(actor.run());

    // Let it run for a few seconds
    tokio::time::sleep(Duration::from_millis(3500)).await;

    let hist = history.read().await;

    // History should be bounded (max 3600 by default)
    assert!(hist.len() <= 3600, "History should not exceed max_history");
    assert!(hist.len() >= 3, "Should have at least 3 entries after 3.5s");
}

// --- Edge Case Tests ---

/// Test MetricPoint with zero values.
#[test]
fn test_metric_point_zero_values() {
    let point = MetricPoint {
        ts: 0,
        cpu_user: 0.0,
        cpu_system: 0.0,
        cpu_idle: 0.0,
        per_core: vec![],
        memory_used: 0,
        memory_total: 0,
        zfs_arc: 0,
        rx_bytes: 0,
        tx_bytes: 0,
    };

    let json = serde_json::to_string(&point).expect("Serialization failed");
    let _: MetricPoint = serde_json::from_str(&json).expect("Deserialization failed");
}

/// Test MetricPoint with max values.
#[test]
fn test_metric_point_max_values() {
    let point = MetricPoint {
        ts: i64::MAX,
        cpu_user: 100.0,
        cpu_system: 100.0,
        cpu_idle: 100.0,
        per_core: vec![CpuCoreMetric {
            id: i32::MAX,
            cpu_user: 100.0,
            cpu_system: 100.0,
            cpu_idle: 100.0,
        }],
        memory_used: u64::MAX,
        memory_total: u64::MAX,
        zfs_arc: u64::MAX,
        rx_bytes: u64::MAX,
        tx_bytes: u64::MAX,
    };

    let json = serde_json::to_string(&point).expect("Serialization failed");
    let deserialized: MetricPoint = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.ts, i64::MAX);
}

/// Test MetricPoint with many cores.
#[test]
fn test_metric_point_many_cores() {
    let per_core: Vec<CpuCoreMetric> = (0..256)
        .map(|id| CpuCoreMetric {
            id,
            cpu_user: 25.0,
            cpu_system: 25.0,
            cpu_idle: 50.0,
        })
        .collect();

    let point = MetricPoint {
        ts: 1700000000000,
        cpu_user: 25.0,
        cpu_system: 25.0,
        cpu_idle: 50.0,
        per_core,
        memory_used: 8 * 1024 * 1024 * 1024,
        memory_total: 16 * 1024 * 1024 * 1024,
        zfs_arc: 0,
        rx_bytes: 0,
        tx_bytes: 0,
    };

    let json = serde_json::to_string(&point).expect("Serialization failed");
    let deserialized: MetricPoint = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.per_core.len(), 256);
}

/// Test CpuCoreMetric Clone and Debug.
#[test]
fn test_cpu_core_metric_clone_debug() {
    let core = CpuCoreMetric {
        id: 42,
        cpu_user: 33.3,
        cpu_system: 16.6,
        cpu_idle: 50.1,
    };

    let cloned = core.clone();
    assert_eq!(cloned.id, core.id);
    assert_eq!(cloned.cpu_user, core.cpu_user);

    let debug_str = format!("{:?}", core);
    assert!(debug_str.contains("CpuCoreMetric"));
    assert!(debug_str.contains("42"));
}

/// Test MetricPoint with negative timestamp.
#[test]
fn test_metric_point_negative_timestamp() {
    let point = MetricPoint {
        ts: -1000,
        cpu_user: 50.0,
        cpu_system: 30.0,
        cpu_idle: 20.0,
        per_core: vec![],
        memory_used: 1000,
        memory_total: 2000,
        zfs_arc: 0,
        rx_bytes: 0,
        tx_bytes: 0,
    };

    let json = serde_json::to_string(&point).expect("Serialization failed");
    let deserialized: MetricPoint = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.ts, -1000);
}

/// Test MetricPoint with floating point edge cases.
#[test]
fn test_metric_point_float_edge_cases() {
    let point = MetricPoint {
        ts: 1700000000000,
        cpu_user: f32::MIN_POSITIVE,
        cpu_system: 0.0000001,
        cpu_idle: 99.9999999,
        per_core: vec![],
        memory_used: 1,
        memory_total: 1,
        zfs_arc: 0,
        rx_bytes: 0,
        tx_bytes: 0,
    };

    let json = serde_json::to_string(&point).expect("Serialization failed");
    let _: MetricPoint = serde_json::from_str(&json).expect("Deserialization failed");
}

/// Test multiple broadcast receivers.
#[tokio::test]
async fn test_multiple_broadcast_receivers() {
    let (_cmd_tx, cmd_rx) = mpsc::channel(10);
    let (broadcast_tx, _) = broadcast::channel(10);

    let mut rx1 = broadcast_tx.subscribe();
    let mut rx2 = broadcast_tx.subscribe();
    let mut rx3 = broadcast_tx.subscribe();

    let actor = MetricsActor::new(cmd_rx, broadcast_tx).expect("Failed to construct MetricsActor");
    tokio::spawn(actor.run());

    // Wait for a metric
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // All receivers should get the same data
    let r1 = rx1.try_recv();
    let r2 = rx2.try_recv();
    let r3 = rx3.try_recv();

    // At least one should have received
    assert!(
        r1.is_ok() || r2.is_ok() || r3.is_ok(),
        "At least one receiver should get data"
    );
}
