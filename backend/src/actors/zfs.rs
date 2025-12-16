//! ZFS Actor for sequential access to ZFS operations.
//!
//! This module implements the Actor pattern for ZFS operations to ensure
//! thread-safe, sequential access to libzfs. All ZFS operations should go
//! through this actor to avoid concurrent access issues.
//!
//! # Usage
//! ```no_run
//! use yanos_backend::actors::start_zfs_actor;
//! use yanos_backend::error::AppError;
//!
//! async fn example() -> Result<(), AppError> {
//!     let zfs_actor = start_zfs_actor()?;
//!     let pools = zfs_actor.list_pools().await?;
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, instrument};

use crate::adapters::zfs::{DatasetInfo, LibZfsHandle, PoolInfo};
use crate::error::AppError;

/// Messages that can be sent to the ZFS actor.
#[derive(Debug)]
pub enum ZfsMessage {
    /// List all pools
    ListPools {
        resp: oneshot::Sender<Result<Vec<PoolInfo>, AppError>>,
    },
    /// Get a specific pool by name
    GetPool {
        name: String,
        resp: oneshot::Sender<Result<PoolInfo, AppError>>,
    },
    /// List all datasets in a pool
    ListDatasets {
        pool: String,
        resp: oneshot::Sender<Result<Vec<DatasetInfo>, AppError>>,
    },
    /// Get a specific dataset
    GetDataset {
        name: String,
        resp: oneshot::Sender<Result<DatasetInfo, AppError>>,
    },
}

/// Handle to communicate with the ZFS actor.
#[derive(Clone, Debug)]
pub struct ZfsActorHandle {
    tx: mpsc::Sender<ZfsMessage>,
}

impl ZfsActorHandle {
    /// List all ZFS pools.
    #[instrument(skip(self))]
    pub async fn list_pools(&self) -> Result<Vec<PoolInfo>, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(ZfsMessage::ListPools { resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("ZFS actor unavailable: {e}")))?;
        rx.await
            .map_err(|e| AppError::ServiceUnavailable(format!("ZFS actor channel closed: {e}")))?
    }

    /// Get a specific pool by name.
    #[instrument(skip(self))]
    pub async fn get_pool(&self, name: String) -> Result<PoolInfo, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(ZfsMessage::GetPool { name, resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("ZFS actor unavailable: {e}")))?;
        rx.await
            .map_err(|e| AppError::ServiceUnavailable(format!("ZFS actor channel closed: {e}")))?
    }

    /// List all datasets in a pool.
    #[instrument(skip(self))]
    pub async fn list_datasets(&self, pool: String) -> Result<Vec<DatasetInfo>, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(ZfsMessage::ListDatasets { pool, resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("ZFS actor unavailable: {e}")))?;
        rx.await
            .map_err(|e| AppError::ServiceUnavailable(format!("ZFS actor channel closed: {e}")))?
    }

    /// Get a specific dataset.
    #[instrument(skip(self))]
    pub async fn get_dataset(&self, name: String) -> Result<DatasetInfo, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(ZfsMessage::GetDataset { name, resp })
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("ZFS actor unavailable: {e}")))?;
        rx.await
            .map_err(|e| AppError::ServiceUnavailable(format!("ZFS actor channel closed: {e}")))?
    }
}

/// Start the ZFS actor and return a handle.
///
/// The actor maintains a single libzfs handle and processes all ZFS
/// operations sequentially to ensure thread safety.
pub fn start_zfs_actor() -> Result<ZfsActorHandle, AppError> {
    let (tx, mut rx) = mpsc::channel::<ZfsMessage>(32);

    // Initialize libzfs handle
    let handle = Arc::new(LibZfsHandle::new()?);

    info!(target: "yanos::zfs_actor", "ZFS actor started");

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let handle = Arc::clone(&handle);

            // Process messages on a blocking thread since libzfs may block
            let result = tokio::task::spawn_blocking(move || match msg {
                ZfsMessage::ListPools { resp } => {
                    let result = crate::adapters::zfs::list_pools(&handle);
                    let _ = resp.send(result);
                }
                ZfsMessage::GetPool { name, resp } => {
                    let result = crate::adapters::zfs::get_pool(&handle, &name);
                    let _ = resp.send(result);
                }
                ZfsMessage::ListDatasets { pool, resp } => {
                    let result = crate::adapters::zfs::list_datasets(&handle, &pool);
                    let _ = resp.send(result);
                }
                ZfsMessage::GetDataset { name, resp } => {
                    let result = crate::adapters::zfs::get_dataset(&handle, &name);
                    let _ = resp.send(result);
                }
            })
            .await;

            if let Err(e) = result {
                error!(target: "yanos::zfs_actor", error = ?e, "ZFS operation panicked");
            }
        }

        info!(target: "yanos::zfs_actor", "ZFS actor shutting down");
    });

    Ok(ZfsActorHandle { tx })
}

/// Start a mock ZFS actor for testing when libzfs is unavailable.
///
/// Returns empty lists for all queries. Used in tests on systems
/// without ZFS support.
pub fn start_mock_zfs_actor() -> ZfsActorHandle {
    let (tx, mut rx) = mpsc::channel::<ZfsMessage>(32);

    info!(target: "yanos::zfs_actor", "Starting mock ZFS actor (libzfs unavailable)");

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                ZfsMessage::ListPools { resp } => {
                    let _ = resp.send(Ok(vec![]));
                }
                ZfsMessage::GetPool { name, resp } => {
                    let _ = resp.send(Err(crate::error::AppError::NotFound(format!(
                        "Pool '{}' not found (mock mode)",
                        name
                    ))));
                }
                ZfsMessage::ListDatasets { pool, resp } => {
                    let _ = resp.send(Err(crate::error::AppError::NotFound(format!(
                        "Pool '{}' not found (mock mode)",
                        pool
                    ))));
                }
                ZfsMessage::GetDataset { name, resp } => {
                    let _ = resp.send(Err(crate::error::AppError::NotFound(format!(
                        "Dataset '{}' not found (mock mode)",
                        name
                    ))));
                }
            }
        }
    });

    ZfsActorHandle { tx }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_zfs_actor_handle_clone() {
        let actor = start_zfs_actor().expect("Failed to start ZFS actor");
        let cloned = actor.clone();

        // Both handles should work
        let result1 = actor.list_pools().await;
        let result2 = cloned.list_pools().await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_zfs_message_debug() {
        // Test that messages implement Debug
        let (tx, _) = oneshot::channel();
        let msg = ZfsMessage::ListPools { resp: tx };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("ListPools"));
    }

    #[test]
    fn test_zfs_message_get_pool_debug() {
        let (tx, _) = oneshot::channel();
        let msg = ZfsMessage::GetPool {
            name: "rpool".to_string(),
            resp: tx,
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("GetPool"));
        assert!(debug_str.contains("rpool"));
    }

    #[test]
    fn test_zfs_message_list_datasets_debug() {
        let (tx, _) = oneshot::channel();
        let msg = ZfsMessage::ListDatasets {
            pool: "rpool".to_string(),
            resp: tx,
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("ListDatasets"));
    }

    #[test]
    fn test_zfs_message_get_dataset_debug() {
        let (tx, _) = oneshot::channel();
        let msg = ZfsMessage::GetDataset {
            name: "rpool/ROOT".to_string(),
            resp: tx,
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("GetDataset"));
    }

    #[tokio::test]
    async fn test_zfs_actor_list_pools() {
        let actor = start_zfs_actor().expect("Failed to start ZFS actor");
        let pools = actor.list_pools().await.expect("Failed to list pools");

        // On OmniOS, we should have at least rpool
        assert!(!pools.is_empty(), "Expected at least one pool");

        // Verify pool data is populated
        let first = &pools[0];
        assert!(!first.name.is_empty());
        assert!(!first.health.is_empty());
    }

    #[tokio::test]
    async fn test_zfs_actor_get_pool() {
        let actor = start_zfs_actor().expect("Failed to start ZFS actor");
        let pools = actor.list_pools().await.expect("Failed to list pools");

        let first_pool_name = pools[0].name.clone();
        let pool = actor
            .get_pool(first_pool_name.clone())
            .await
            .expect("Failed to get pool");

        assert_eq!(pool.name, first_pool_name);
        assert!(pool.size > 0);
    }

    #[tokio::test]
    async fn test_zfs_actor_get_pool_not_found() {
        let actor = start_zfs_actor().expect("Failed to start ZFS actor");
        let result = actor.get_pool("nonexistent_pool_xyz".to_string()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::NotFound(_) => {}
            other => panic!("Expected NotFound error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_zfs_actor_list_datasets() {
        let actor = start_zfs_actor().expect("Failed to start ZFS actor");
        let pools = actor.list_pools().await.expect("Failed to list pools");

        let pool_name = pools[0].name.clone();
        let datasets = actor
            .list_datasets(pool_name.clone())
            .await
            .expect("Failed to list datasets");

        // Should have at least the root dataset
        assert!(!datasets.is_empty());

        // All datasets should belong to this pool
        for ds in &datasets {
            assert!(
                ds.name.starts_with(&pool_name),
                "Dataset {} should start with pool name {}",
                ds.name,
                pool_name
            );
        }
    }

    #[tokio::test]
    async fn test_zfs_actor_get_dataset() {
        let actor = start_zfs_actor().expect("Failed to start ZFS actor");
        let pools = actor.list_pools().await.expect("Failed to list pools");

        let pool_name = pools[0].name.clone();
        let datasets = actor
            .list_datasets(pool_name.clone())
            .await
            .expect("Failed to list datasets");

        // Get the first dataset
        let ds_name = datasets[0].name.clone();
        let dataset = actor
            .get_dataset(ds_name.clone())
            .await
            .expect("Failed to get dataset");

        assert_eq!(dataset.name, ds_name);
    }

    #[tokio::test]
    async fn test_zfs_actor_get_dataset_not_found() {
        let actor = start_zfs_actor().expect("Failed to start ZFS actor");
        let result = actor
            .get_dataset("rpool/nonexistent_dataset_xyz_123".to_string())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_zfs_actor_concurrent_requests() {
        let actor = start_zfs_actor().expect("Failed to start ZFS actor");

        // Send multiple concurrent requests
        let handle1 = actor.clone();
        let handle2 = actor.clone();
        let handle3 = actor.clone();

        let (r1, r2, r3) = tokio::join!(
            handle1.list_pools(),
            handle2.list_pools(),
            handle3.list_pools(),
        );

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(r3.is_ok());

        // All should return the same data
        assert_eq!(r1.unwrap().len(), r2.unwrap().len());
    }
}
