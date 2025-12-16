//! Native ZFS Integration Tests.
//!
//! These tests require a live OmniOS/illumos environment with libzfs.
//! They interface directly with `libzfs` and the `ZfsActor`.

use yanos_backend::actors::start_zfs_actor;
use yanos_backend::adapters::zfs::{get_dataset, get_pool, list_datasets, list_pools, LibZfsHandle};
use yanos_backend::error::AppError;

// =============================================================================
// LibZfsHandle Tests
// =============================================================================

/// Test libzfs handle initialization.
#[test]
fn test_libzfs_init() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    assert!(!handle.as_ptr().is_null());
}

/// Test that multiple libzfs handles can coexist.
#[test]
fn test_multiple_libzfs_handles() {
    let handle1 = LibZfsHandle::new().expect("Failed to initialize first libzfs handle");
    let handle2 = LibZfsHandle::new().expect("Failed to initialize second libzfs handle");

    assert!(!handle1.as_ptr().is_null());
    assert!(!handle2.as_ptr().is_null());

    // Both handles should be able to list pools
    let pools1 = list_pools(&handle1).expect("Failed to list pools with handle1");
    let pools2 = list_pools(&handle2).expect("Failed to list pools with handle2");

    assert_eq!(pools1.len(), pools2.len());
}

/// Test libzfs handle last_error when no error has occurred.
#[test]
fn test_libzfs_last_error_no_error() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    // Just verify it doesn't crash
    let _ = handle.last_error();
}

// =============================================================================
// Pool Adapter Tests
// =============================================================================

/// Test listing all pools.
#[test]
fn test_adapter_list_pools() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let pools = list_pools(&handle).expect("Failed to list pools");

    // OmniOS should have at least rpool
    assert!(!pools.is_empty(), "Expected at least one pool on OmniOS");

    for pool in &pools {
        assert!(!pool.name.is_empty(), "Pool name should not be empty");
        assert!(!pool.health.is_empty(), "Pool health should not be empty");
        assert!(!pool.state.is_empty(), "Pool state should not be empty");
        // Size should be > 0 for any real pool
        assert!(pool.size > 0, "Pool size should be greater than 0");
    }
}

/// Test getting a specific pool.
#[test]
fn test_adapter_get_pool() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let pools = list_pools(&handle).expect("Failed to list pools");

    let first_pool = pools.first().expect("Expected at least one pool");
    let pool = get_pool(&handle, &first_pool.name).expect("Failed to get pool");

    assert_eq!(pool.name, first_pool.name);
    assert_eq!(pool.size, first_pool.size);
    assert_eq!(pool.health, first_pool.health);
}

/// Test getting a non-existent pool returns NotFound.
#[test]
fn test_adapter_get_pool_not_found() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let result = get_pool(&handle, "nonexistent_pool_xyz_12345");

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::NotFound(msg) => {
            assert!(msg.contains("nonexistent_pool_xyz_12345"));
        }
        other => panic!("Expected NotFound error, got {:?}", other),
    }
}

/// Test pool properties are reasonable.
#[test]
fn test_adapter_pool_properties() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let pools = list_pools(&handle).expect("Failed to list pools");
    let pool = pools.first().expect("Expected at least one pool");

    // Capacity should be 0-100
    assert!(pool.capacity <= 100, "Capacity should be <= 100");

    // Free + allocated should approximately equal size (may not be exact due to overhead)
    let total = pool.free + pool.allocated;
    assert!(
        total <= pool.size + 1024 * 1024,
        "Free + allocated should be close to size"
    );

    // Health should be a known value
    let valid_health = ["ONLINE", "DEGRADED", "FAULTED", "OFFLINE", "UNAVAIL", "REMOVED"];
    assert!(
        valid_health.contains(&pool.health.as_str()),
        "Health '{}' should be a known value",
        pool.health
    );
}

// =============================================================================
// Dataset Adapter Tests
// =============================================================================

/// Test listing datasets in a pool.
#[test]
fn test_adapter_list_datasets() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let pools = list_pools(&handle).expect("Failed to list pools");
    let pool = pools.first().expect("Expected at least one pool");

    let datasets = list_datasets(&handle, &pool.name).expect("Failed to list datasets");

    // Should have at least the root dataset
    assert!(
        !datasets.is_empty(),
        "Expected at least one dataset in pool"
    );

    // First dataset should be the pool root
    assert_eq!(datasets[0].name, pool.name);

    for ds in &datasets {
        assert!(
            ds.name.starts_with(&pool.name),
            "Dataset '{}' should start with pool name '{}'",
            ds.name,
            pool.name
        );
        assert_eq!(ds.pool, pool.name);
        assert!(!ds.dataset_type.is_empty());
        assert!(!ds.compression.is_empty());
    }
}

/// Test listing datasets for non-existent pool returns NotFound.
#[test]
fn test_adapter_list_datasets_not_found() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let result = list_datasets(&handle, "nonexistent_pool_xyz_12345");

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::NotFound(_) => {}
        other => panic!("Expected NotFound error, got {:?}", other),
    }
}

/// Test getting a specific dataset.
#[test]
fn test_adapter_get_dataset() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let pools = list_pools(&handle).expect("Failed to list pools");
    let pool = pools.first().expect("Expected at least one pool");

    let datasets = list_datasets(&handle, &pool.name).expect("Failed to list datasets");
    let first_ds = datasets.first().expect("Expected at least one dataset");

    let dataset = get_dataset(&handle, &first_ds.name).expect("Failed to get dataset");

    assert_eq!(dataset.name, first_ds.name);
    assert_eq!(dataset.pool, first_ds.pool);
    assert_eq!(dataset.dataset_type, first_ds.dataset_type);
}

/// Test getting a non-existent dataset returns NotFound.
#[test]
fn test_adapter_get_dataset_not_found() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let result = get_dataset(&handle, "rpool/nonexistent_dataset_xyz_12345");

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::NotFound(_) => {}
        other => panic!("Expected NotFound error, got {:?}", other),
    }
}

/// Test dataset types are correct.
#[test]
fn test_adapter_dataset_types() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let pools = list_pools(&handle).expect("Failed to list pools");
    let pool = pools.first().expect("Expected at least one pool");

    let datasets = list_datasets(&handle, &pool.name).expect("Failed to list datasets");

    let valid_types = ["filesystem", "volume", "snapshot", "bookmark"];
    for ds in &datasets {
        assert!(
            valid_types.contains(&ds.dataset_type.as_str()),
            "Dataset type '{}' should be a known value",
            ds.dataset_type
        );
    }
}

/// Test dataset mountpoint handling.
#[test]
fn test_adapter_dataset_mountpoints() {
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let pools = list_pools(&handle).expect("Failed to list pools");
    let pool = pools.first().expect("Expected at least one pool");

    let datasets = list_datasets(&handle, &pool.name).expect("Failed to list datasets");

    // At least one filesystem dataset should have a mountpoint
    let has_mountpoint = datasets
        .iter()
        .filter(|ds| ds.dataset_type == "filesystem")
        .any(|ds| ds.mountpoint.is_some());

    assert!(
        has_mountpoint,
        "Expected at least one mounted filesystem dataset"
    );
}

// =============================================================================
// ZFS Actor Tests
// =============================================================================

/// Test ZFS actor initialization.
#[tokio::test]
async fn test_actor_init() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    // Actor should be usable
    let result = actor.list_pools().await;
    assert!(result.is_ok());
}

/// Test ZFS actor list_pools.
#[tokio::test]
async fn test_actor_list_pools() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let pools = actor.list_pools().await.expect("Failed to list pools");

    assert!(!pools.is_empty(), "Expected at least one pool");

    // Verify data matches adapter
    let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
    let adapter_pools = list_pools(&handle).expect("Failed to list pools via adapter");

    assert_eq!(pools.len(), adapter_pools.len());
}

/// Test ZFS actor get_pool.
#[tokio::test]
async fn test_actor_get_pool() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let pools = actor.list_pools().await.expect("Failed to list pools");
    let pool_name = pools[0].name.clone();

    let pool = actor
        .get_pool(pool_name.clone())
        .await
        .expect("Failed to get pool");

    assert_eq!(pool.name, pool_name);
}

/// Test ZFS actor get_pool not found.
#[tokio::test]
async fn test_actor_get_pool_not_found() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let result = actor.get_pool("nonexistent_pool_xyz_12345".to_string()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::NotFound(_) => {}
        other => panic!("Expected NotFound error, got {:?}", other),
    }
}

/// Test ZFS actor list_datasets.
#[tokio::test]
async fn test_actor_list_datasets() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let pools = actor.list_pools().await.expect("Failed to list pools");
    let pool_name = pools[0].name.clone();

    let datasets = actor
        .list_datasets(pool_name.clone())
        .await
        .expect("Failed to list datasets");

    assert!(!datasets.is_empty());

    for ds in &datasets {
        assert!(ds.name.starts_with(&pool_name));
    }
}

/// Test ZFS actor list_datasets not found.
#[tokio::test]
async fn test_actor_list_datasets_not_found() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let result = actor
        .list_datasets("nonexistent_pool_xyz_12345".to_string())
        .await;

    assert!(result.is_err());
}

/// Test ZFS actor get_dataset.
#[tokio::test]
async fn test_actor_get_dataset() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let pools = actor.list_pools().await.expect("Failed to list pools");
    let pool_name = pools[0].name.clone();

    let datasets = actor
        .list_datasets(pool_name)
        .await
        .expect("Failed to list datasets");

    let ds_name = datasets[0].name.clone();
    let dataset = actor
        .get_dataset(ds_name.clone())
        .await
        .expect("Failed to get dataset");

    assert_eq!(dataset.name, ds_name);
}

/// Test ZFS actor get_dataset not found.
#[tokio::test]
async fn test_actor_get_dataset_not_found() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let result = actor
        .get_dataset("rpool/nonexistent_dataset_xyz_12345".to_string())
        .await;

    assert!(result.is_err());
}

/// Test ZFS actor handle cloning.
#[tokio::test]
async fn test_actor_handle_clone() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let cloned = actor.clone();

    let result1 = actor.list_pools().await;
    let result2 = cloned.list_pools().await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert_eq!(result1.unwrap().len(), result2.unwrap().len());
}

/// Test ZFS actor concurrent requests.
#[tokio::test]
async fn test_actor_concurrent_requests() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let a = actor.clone();
            tokio::spawn(async move { a.list_pools().await })
        })
        .collect();

    for handle in handles {
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok());
    }
}

/// Test ZFS actor mixed concurrent operations.
#[tokio::test]
async fn test_actor_mixed_concurrent_operations() {
    let actor = start_zfs_actor().expect("Failed to start ZFS actor");
    let pools = actor.list_pools().await.expect("Failed to list pools");
    let pool_name = pools[0].name.clone();

    let a1 = actor.clone();
    let a2 = actor.clone();
    let a3 = actor.clone();
    let pn = pool_name.clone();

    let (r1, r2, r3) = tokio::join!(
        a1.list_pools(),
        a2.get_pool(pool_name.clone()),
        a3.list_datasets(pn),
    );

    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());
}
