//! Tests for ZFS storage management functionality.
//!
//! These tests verify:
//! - ZFS API endpoint behavior
//! - Storage pool listing
//! - Dataset operations
//! - Data structure serialization

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hyper::Method;
use tower::ServiceExt;
use yanos_backend::adapters::zfs::{DatasetInfo, PoolInfo};

// =============================================================================
// API Endpoint Authorization Tests
// =============================================================================

/// Test that storage pools endpoint requires authentication.
#[tokio::test]
async fn test_storage_pools_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/storage/pools")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test that storage datasets endpoint requires authentication.
#[tokio::test]
async fn test_storage_datasets_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/storage/pools/tank/datasets")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test that single pool endpoint requires authentication.
#[tokio::test]
async fn test_storage_pool_by_name_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/storage/pools/tank")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test that dataset by name endpoint requires authentication.
#[tokio::test]
async fn test_storage_dataset_by_name_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/storage/datasets/tank/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test that dataset endpoint with deep path requires authentication.
#[tokio::test]
async fn test_storage_dataset_deep_path_requires_auth() {
    let (app, _, _, _) = common::create_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/storage/datasets/rpool/ROOT/omnios")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// =============================================================================
// PoolInfo Serialization Tests
// =============================================================================

/// Test PoolInfo serialization.
#[test]
fn test_pool_info_serialization() {
    let pool = PoolInfo {
        name: "tank".to_string(),
        size: 1024 * 1024 * 1024 * 100, // 100 GiB
        allocated: 1024 * 1024 * 1024 * 30, // 30 GiB
        free: 1024 * 1024 * 1024 * 70, // 70 GiB
        capacity: 30,
        fragmentation: 5,
        health: "ONLINE".to_string(),
        state: "ACTIVE".to_string(),
        altroot: None,
    };

    let json = serde_json::to_string(&pool).expect("Failed to serialize PoolInfo");
    assert!(json.contains("\"name\":\"tank\""));
    assert!(json.contains("\"health\":\"ONLINE\""));
    assert!(json.contains("\"capacity\":30"));

    let deserialized: PoolInfo =
        serde_json::from_str(&json).expect("Failed to deserialize PoolInfo");
    assert_eq!(deserialized.name, "tank");
    assert_eq!(deserialized.health, "ONLINE");
}

/// Test PoolInfo with altroot.
#[test]
fn test_pool_info_with_altroot() {
    let pool = PoolInfo {
        name: "backup".to_string(),
        size: 500 * 1024 * 1024 * 1024,
        allocated: 100 * 1024 * 1024 * 1024,
        free: 400 * 1024 * 1024 * 1024,
        capacity: 20,
        fragmentation: 2,
        health: "ONLINE".to_string(),
        state: "ACTIVE".to_string(),
        altroot: Some("/mnt/backup".to_string()),
    };

    let json = serde_json::to_string(&pool).expect("Failed to serialize");
    assert!(json.contains("\"/mnt/backup\""));

    let deserialized: PoolInfo = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.altroot, Some("/mnt/backup".to_string()));
}

/// Test pool with special characters in name.
#[test]
fn test_pool_special_name() {
    let pool = PoolInfo {
        name: "tank-backup_2024".to_string(),
        size: 0,
        allocated: 0,
        free: 0,
        capacity: 0,
        fragmentation: 0,
        health: "ONLINE".to_string(),
        state: "ACTIVE".to_string(),
        altroot: None,
    };

    let json = serde_json::to_string(&pool).expect("Failed to serialize");
    let deserialized: PoolInfo = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.name, "tank-backup_2024");
}

/// Test all health states serialize correctly.
#[test]
fn test_all_health_states() {
    let states = ["ONLINE", "DEGRADED", "FAULTED", "OFFLINE", "UNAVAIL", "REMOVED"];

    for state in states {
        let pool = PoolInfo {
            name: "test".to_string(),
            size: 0,
            allocated: 0,
            free: 0,
            capacity: 0,
            fragmentation: 0,
            health: state.to_string(),
            state: "ACTIVE".to_string(),
            altroot: None,
        };

        let json = serde_json::to_string(&pool).expect("Failed to serialize");
        assert!(json.contains(&format!("\"health\":\"{}\"", state)));
    }
}

/// Test all pool states serialize correctly.
#[test]
fn test_all_pool_states() {
    let states = [
        "ACTIVE",
        "EXPORTED",
        "DESTROYED",
        "SPARE",
        "L2CACHE",
        "UNINITIALIZED",
        "UNAVAIL",
        "POTENTIALLY_ACTIVE",
    ];

    for state in states {
        let pool = PoolInfo {
            name: "test".to_string(),
            size: 0,
            allocated: 0,
            free: 0,
            capacity: 0,
            fragmentation: 0,
            health: "ONLINE".to_string(),
            state: state.to_string(),
            altroot: None,
        };

        let json = serde_json::to_string(&pool).expect("Failed to serialize");
        assert!(json.contains(&format!("\"state\":\"{}\"", state)));
    }
}

/// Test PoolInfo clone.
#[test]
fn test_pool_info_clone() {
    let pool = PoolInfo {
        name: "rpool".to_string(),
        size: 100,
        allocated: 50,
        free: 50,
        capacity: 50,
        fragmentation: 10,
        health: "ONLINE".to_string(),
        state: "ACTIVE".to_string(),
        altroot: Some("/alt".to_string()),
    };

    let cloned = pool.clone();
    assert_eq!(cloned.name, pool.name);
    assert_eq!(cloned.size, pool.size);
    assert_eq!(cloned.altroot, pool.altroot);
}

/// Test PoolInfo debug.
#[test]
fn test_pool_info_debug() {
    let pool = PoolInfo {
        name: "test".to_string(),
        size: 0,
        allocated: 0,
        free: 0,
        capacity: 0,
        fragmentation: 0,
        health: "ONLINE".to_string(),
        state: "ACTIVE".to_string(),
        altroot: None,
    };

    let debug_str = format!("{:?}", pool);
    assert!(debug_str.contains("PoolInfo"));
    assert!(debug_str.contains("test"));
}

// =============================================================================
// DatasetInfo Serialization Tests
// =============================================================================

/// Test DatasetInfo serialization.
#[test]
fn test_dataset_info_serialization() {
    let dataset = DatasetInfo {
        name: "tank/data".to_string(),
        pool: "tank".to_string(),
        dataset_type: "filesystem".to_string(),
        used: 1024 * 1024 * 1024 * 10, // 10 GiB
        available: 1024 * 1024 * 1024 * 60, // 60 GiB
        referenced: 1024 * 1024 * 1024 * 10,
        compressratio: 150, // 1.50x
        mountpoint: Some("/data".to_string()),
        compression: "lz4".to_string(),
    };

    let json = serde_json::to_string(&dataset).expect("Failed to serialize DatasetInfo");
    assert!(json.contains("\"name\":\"tank/data\""));
    assert!(json.contains("\"compression\":\"lz4\""));

    let deserialized: DatasetInfo =
        serde_json::from_str(&json).expect("Failed to deserialize DatasetInfo");
    assert_eq!(deserialized.name, "tank/data");
    assert_eq!(deserialized.compression, "lz4");
    assert_eq!(deserialized.mountpoint, Some("/data".to_string()));
}

/// Test DatasetInfo with no mountpoint.
#[test]
fn test_dataset_info_no_mountpoint() {
    let dataset = DatasetInfo {
        name: "tank/zvol".to_string(),
        pool: "tank".to_string(),
        dataset_type: "volume".to_string(),
        used: 1024 * 1024 * 1024,
        available: 1024 * 1024 * 1024 * 99,
        referenced: 1024 * 1024 * 1024,
        compressratio: 100,
        mountpoint: None,
        compression: "off".to_string(),
    };

    let json = serde_json::to_string(&dataset).expect("Failed to serialize DatasetInfo");
    assert!(json.contains("\"mountpoint\":null"));
}

/// Test dataset with deep path.
#[test]
fn test_dataset_deep_path() {
    let dataset = DatasetInfo {
        name: "tank/data/users/home/documents".to_string(),
        pool: "tank".to_string(),
        dataset_type: "filesystem".to_string(),
        used: 0,
        available: 0,
        referenced: 0,
        compressratio: 100,
        mountpoint: Some("/data/users/home/documents".to_string()),
        compression: "zstd".to_string(),
    };

    let json = serde_json::to_string(&dataset).expect("Failed to serialize");
    let deserialized: DatasetInfo = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.name, "tank/data/users/home/documents");
}

/// Test all dataset types serialize correctly.
#[test]
fn test_all_dataset_types() {
    let types = ["filesystem", "volume", "snapshot", "bookmark"];

    for ds_type in types {
        let dataset = DatasetInfo {
            name: "test/data".to_string(),
            pool: "test".to_string(),
            dataset_type: ds_type.to_string(),
            used: 0,
            available: 0,
            referenced: 0,
            compressratio: 100,
            mountpoint: None,
            compression: "off".to_string(),
        };

        let json = serde_json::to_string(&dataset).expect("Failed to serialize");
        assert!(json.contains(&format!("\"dataset_type\":\"{}\"", ds_type)));
    }
}

/// Test DatasetInfo clone.
#[test]
fn test_dataset_info_clone() {
    let ds = DatasetInfo {
        name: "test/data".to_string(),
        pool: "test".to_string(),
        dataset_type: "filesystem".to_string(),
        used: 100,
        available: 900,
        referenced: 100,
        compressratio: 100,
        mountpoint: Some("/data".to_string()),
        compression: "lz4".to_string(),
    };

    let cloned = ds.clone();
    assert_eq!(cloned.name, ds.name);
    assert_eq!(cloned.mountpoint, ds.mountpoint);
}

/// Test DatasetInfo debug.
#[test]
fn test_dataset_info_debug() {
    let ds = DatasetInfo {
        name: "rpool/ROOT".to_string(),
        pool: "rpool".to_string(),
        dataset_type: "filesystem".to_string(),
        used: 0,
        available: 0,
        referenced: 0,
        compressratio: 100,
        mountpoint: Some("/".to_string()),
        compression: "on".to_string(),
    };

    let debug_str = format!("{:?}", ds);
    assert!(debug_str.contains("DatasetInfo"));
    assert!(debug_str.contains("rpool/ROOT"));
}

/// Test compression algorithms.
#[test]
fn test_compression_algorithms() {
    let algorithms = ["off", "on", "lz4", "zstd", "gzip", "gzip-9", "lzjb", "zle"];

    for algo in algorithms {
        let ds = DatasetInfo {
            name: "test/data".to_string(),
            pool: "test".to_string(),
            dataset_type: "filesystem".to_string(),
            used: 0,
            available: 0,
            referenced: 0,
            compressratio: 100,
            mountpoint: None,
            compression: algo.to_string(),
        };

        let json = serde_json::to_string(&ds).expect("Failed to serialize");
        assert!(json.contains(&format!("\"compression\":\"{}\"", algo)));
    }
}

/// Test large values for pool sizes.
#[test]
fn test_large_pool_values() {
    // Test petabyte-scale pool
    let pool = PoolInfo {
        name: "bigpool".to_string(),
        size: 1024 * 1024 * 1024 * 1024 * 1024, // 1 PiB
        allocated: 500 * 1024 * 1024 * 1024 * 1024, // 500 TiB
        free: 524 * 1024 * 1024 * 1024 * 1024, // ~524 TiB
        capacity: 50,
        fragmentation: 1,
        health: "ONLINE".to_string(),
        state: "ACTIVE".to_string(),
        altroot: None,
    };

    let json = serde_json::to_string(&pool).expect("Failed to serialize large pool");
    let deserialized: PoolInfo = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.size, pool.size);
    assert!(deserialized.size > 1024 * 1024 * 1024 * 1024 * 1024 - 1);
}

/// Test zero values.
#[test]
fn test_zero_values() {
    let pool = PoolInfo {
        name: "empty".to_string(),
        size: 0,
        allocated: 0,
        free: 0,
        capacity: 0,
        fragmentation: 0,
        health: "ONLINE".to_string(),
        state: "EXPORTED".to_string(),
        altroot: None,
    };

    let json = serde_json::to_string(&pool).expect("Failed to serialize");
    let deserialized: PoolInfo = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.size, 0);
    assert_eq!(deserialized.capacity, 0);
}

/// Test JSON round-trip preserves all fields.
#[test]
fn test_json_round_trip() {
    let pool = PoolInfo {
        name: "roundtrip".to_string(),
        size: 12345678901234,
        allocated: 1234567890123,
        free: 11111111011111,
        capacity: 10,
        fragmentation: 3,
        health: "DEGRADED".to_string(),
        state: "ACTIVE".to_string(),
        altroot: Some("/mnt/special".to_string()),
    };

    let json = serde_json::to_string(&pool).expect("Failed to serialize");
    let deserialized: PoolInfo = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.name, pool.name);
    assert_eq!(deserialized.size, pool.size);
    assert_eq!(deserialized.allocated, pool.allocated);
    assert_eq!(deserialized.free, pool.free);
    assert_eq!(deserialized.capacity, pool.capacity);
    assert_eq!(deserialized.fragmentation, pool.fragmentation);
    assert_eq!(deserialized.health, pool.health);
    assert_eq!(deserialized.state, pool.state);
    assert_eq!(deserialized.altroot, pool.altroot);
}
