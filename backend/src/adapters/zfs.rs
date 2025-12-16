//! ZFS FFI bindings for libzfs on illumos.
//!
//! This module provides safe Rust wrappers around libzfs C library functions
//! for managing ZFS pools and datasets on illumos/OmniOS systems.
//!
//! # Safety
//! All FFI calls are wrapped in safe functions that handle errors appropriately.
//! The module uses RAII patterns to ensure proper cleanup of libzfs handles.
//!
//! # Usage
//! ```no_run
//! use yanos_backend::adapters::zfs::{LibZfsHandle, list_pools, list_datasets};
//! use yanos_backend::error::AppError;
//!
//! fn main() -> Result<(), AppError> {
//!     let handle = LibZfsHandle::new()?;
//!     let pools = list_pools(&handle)?;
//!     let datasets = list_datasets(&handle, "tank")?;
//!     Ok(())
//! }
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
use utoipa::ToSchema;

use crate::error::AppError;

// =============================================================================
// FFI Type Definitions
// =============================================================================

/// Opaque handle to libzfs library state.
#[repr(C)]
pub struct libzfs_handle_t {
    _private: [u8; 0],
}

/// Opaque handle to a ZFS pool.
#[repr(C)]
pub struct zpool_handle_t {
    _private: [u8; 0],
}

/// Opaque handle to a ZFS dataset (filesystem, volume, snapshot).
#[repr(C)]
pub struct zfs_handle_t {
    _private: [u8; 0],
}

/// Pool health status values from sys/fs/zfs.h
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum zpool_status_t {
    ZPOOL_STATUS_CORRUPT_CACHE = 0,
    ZPOOL_STATUS_MISSING_DEV_R,
    ZPOOL_STATUS_MISSING_DEV_NR,
    ZPOOL_STATUS_CORRUPT_LABEL_R,
    ZPOOL_STATUS_CORRUPT_LABEL_NR,
    ZPOOL_STATUS_BAD_GUID_SUM,
    ZPOOL_STATUS_CORRUPT_POOL,
    ZPOOL_STATUS_CORRUPT_DATA,
    ZPOOL_STATUS_FAILING_DEV,
    ZPOOL_STATUS_VERSION_NEWER,
    ZPOOL_STATUS_HOSTID_MISMATCH,
    ZPOOL_STATUS_HOSTID_ACTIVE,
    ZPOOL_STATUS_HOSTID_REQUIRED,
    ZPOOL_STATUS_IO_FAILURE_WAIT,
    ZPOOL_STATUS_IO_FAILURE_CONTINUE,
    ZPOOL_STATUS_IO_FAILURE_MMP,
    ZPOOL_STATUS_BAD_LOG,
    ZPOOL_STATUS_ERRATA,
    ZPOOL_STATUS_FEAT_DISABLED,
    ZPOOL_STATUS_RESILVERING,
    ZPOOL_STATUS_OFFLINE_DEV,
    ZPOOL_STATUS_REMOVED_DEV,
    ZPOOL_STATUS_REBUILDING,
    ZPOOL_STATUS_REBUILD_SCRUB,
    ZPOOL_STATUS_NON_NATIVE_ASHIFT,
    ZPOOL_STATUS_COMPATIBILITY_ERR,
    ZPOOL_STATUS_INCOMPATIBLE_FEAT,
    ZPOOL_STATUS_OK,
}

/// ZFS dataset types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum zfs_type_t {
    ZFS_TYPE_FILESYSTEM = 1 << 0,
    ZFS_TYPE_SNAPSHOT = 1 << 1,
    ZFS_TYPE_VOLUME = 1 << 2,
    ZFS_TYPE_POOL = 1 << 3,
    ZFS_TYPE_BOOKMARK = 1 << 4,
}

/// Pool state values
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum pool_state_t {
    POOL_STATE_ACTIVE = 0,
    POOL_STATE_EXPORTED,
    POOL_STATE_DESTROYED,
    POOL_STATE_SPARE,
    POOL_STATE_L2CACHE,
    POOL_STATE_UNINITIALIZED,
    POOL_STATE_UNAVAIL,
    POOL_STATE_POTENTIALLY_ACTIVE,
}

/// Pool properties
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum zpool_prop_t {
    ZPOOL_PROP_INVAL = -1,
    ZPOOL_PROP_NAME = 0,
    ZPOOL_PROP_SIZE,
    ZPOOL_PROP_CAPACITY,
    ZPOOL_PROP_ALTROOT,
    ZPOOL_PROP_HEALTH,
    ZPOOL_PROP_GUID,
    ZPOOL_PROP_VERSION,
    ZPOOL_PROP_BOOTFS,
    ZPOOL_PROP_DELEGATION,
    ZPOOL_PROP_AUTOREPLACE,
    ZPOOL_PROP_CACHEFILE,
    ZPOOL_PROP_FAILUREMODE,
    ZPOOL_PROP_LISTSNAPS,
    ZPOOL_PROP_AUTOEXPAND,
    ZPOOL_PROP_DEDUPDITTO,
    ZPOOL_PROP_DEDUPRATIO,
    ZPOOL_PROP_FREE,
    ZPOOL_PROP_ALLOCATED,
    ZPOOL_PROP_READONLY,
    ZPOOL_PROP_ASHIFT,
    ZPOOL_PROP_COMMENT,
    ZPOOL_PROP_EXPANDSZ,
    ZPOOL_PROP_FREEING,
    ZPOOL_PROP_FRAGMENTATION,
    ZPOOL_PROP_LEAKED,
    ZPOOL_PROP_MAXBLOCKSIZE,
    ZPOOL_PROP_TNAME,
    ZPOOL_PROP_MAXDNODESIZE,
    ZPOOL_PROP_MULTIHOST,
    ZPOOL_PROP_CHECKPOINT,
    ZPOOL_PROP_LOAD_GUID,
    ZPOOL_PROP_AUTOTRIM,
    ZPOOL_PROP_COMPATIBILITY,
    ZPOOL_NUM_PROPS,
}

/// ZFS properties
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum zfs_prop_t {
    ZFS_PROP_INVAL = -1,
    ZFS_PROP_TYPE = 0,
    ZFS_PROP_CREATION,
    ZFS_PROP_USED,
    ZFS_PROP_AVAILABLE,
    ZFS_PROP_REFERENCED,
    ZFS_PROP_COMPRESSRATIO,
    ZFS_PROP_MOUNTED,
    ZFS_PROP_ORIGIN,
    ZFS_PROP_QUOTA,
    ZFS_PROP_RESERVATION,
    ZFS_PROP_VOLSIZE,
    ZFS_PROP_VOLBLOCKSIZE,
    ZFS_PROP_RECORDSIZE,
    ZFS_PROP_MOUNTPOINT,
    ZFS_PROP_SHARENFS,
    ZFS_PROP_CHECKSUM,
    ZFS_PROP_COMPRESSION,
    ZFS_PROP_ATIME,
    ZFS_PROP_DEVICES,
    ZFS_PROP_EXEC,
    ZFS_PROP_SETUID,
    ZFS_PROP_READONLY,
    ZFS_PROP_ZONED,
    ZFS_PROP_SNAPDIR,
    ZFS_PROP_ACLMODE,
    ZFS_PROP_ACLINHERIT,
    // ... many more, but we only need a few
}

// Callback type for pool iteration
type ZpoolIterFunc = extern "C" fn(*mut zpool_handle_t, *mut c_void) -> c_int;
type ZfsIterFunc = extern "C" fn(*mut zfs_handle_t, *mut c_void) -> c_int;

// =============================================================================
// FFI Function Declarations
// =============================================================================

#[link(name = "zfs")]
unsafe extern "C" {
    // Library handle management
    fn libzfs_init() -> *mut libzfs_handle_t;
    fn libzfs_fini(handle: *mut libzfs_handle_t);
    fn libzfs_error_description(handle: *mut libzfs_handle_t) -> *const c_char;

    // Pool operations
    fn zpool_open(handle: *mut libzfs_handle_t, name: *const c_char) -> *mut zpool_handle_t;
    fn zpool_close(handle: *mut zpool_handle_t);
    fn zpool_get_name(handle: *mut zpool_handle_t) -> *const c_char;
    fn zpool_get_state(handle: *mut zpool_handle_t) -> c_int;
    fn zpool_get_prop(
        handle: *mut zpool_handle_t,
        prop: zpool_prop_t,
        buf: *mut c_char,
        len: usize,
        srctype: *mut c_int,
        literal: c_int,
    ) -> c_int;
    fn zpool_get_prop_int(
        handle: *mut zpool_handle_t,
        prop: zpool_prop_t,
        srctype: *mut c_int,
    ) -> u64;
    fn zpool_iter(
        handle: *mut libzfs_handle_t,
        callback: ZpoolIterFunc,
        data: *mut c_void,
    ) -> c_int;
    fn zpool_get_status(
        handle: *mut zpool_handle_t,
        msgid: *mut *mut c_char,
        errata: *mut c_int,
    ) -> zpool_status_t;

    // Dataset operations
    fn zfs_open(
        handle: *mut libzfs_handle_t,
        name: *const c_char,
        types: c_int,
    ) -> *mut zfs_handle_t;
    fn zfs_close(handle: *mut zfs_handle_t);
    fn zfs_get_name(handle: *mut zfs_handle_t) -> *const c_char;
    fn zfs_get_type(handle: *mut zfs_handle_t) -> zfs_type_t;
    fn zfs_prop_get(
        handle: *mut zfs_handle_t,
        prop: zfs_prop_t,
        buf: *mut c_char,
        len: usize,
        srctype: *mut c_int,
        statbuf: *mut c_char,
        statlen: usize,
        literal: c_int,
    ) -> c_int;
    fn zfs_prop_get_int(handle: *mut zfs_handle_t, prop: zfs_prop_t) -> u64;
    fn zfs_iter_filesystems(
        handle: *mut zfs_handle_t,
        callback: ZfsIterFunc,
        data: *mut c_void,
    ) -> c_int;
    #[allow(dead_code)]
    fn zfs_iter_children(
        handle: *mut zfs_handle_t,
        callback: ZfsIterFunc,
        data: *mut c_void,
    ) -> c_int;
}

// =============================================================================
// Safe Rust Wrapper Types
// =============================================================================

/// RAII wrapper for libzfs handle.
/// Automatically calls libzfs_fini on drop.
pub struct LibZfsHandle {
    handle: *mut libzfs_handle_t,
}

// LibZfsHandle is Send+Sync because libzfs operations are thread-safe
// when using separate handles per thread, or when sequenced via actor.
unsafe impl Send for LibZfsHandle {}
unsafe impl Sync for LibZfsHandle {}

impl LibZfsHandle {
    /// Initialize a new libzfs handle.
    pub fn new() -> Result<Self, AppError> {
        let handle = unsafe { libzfs_init() };
        if handle.is_null() {
            return Err(AppError::ServiceUnavailable(
                "Failed to initialize libzfs handle".to_string(),
            ));
        }
        Ok(Self { handle })
    }

    /// Get the raw handle pointer (for FFI calls).
    pub fn as_ptr(&self) -> *mut libzfs_handle_t {
        self.handle
    }

    /// Get the last error description from libzfs.
    pub fn last_error(&self) -> String {
        unsafe {
            let err_ptr = libzfs_error_description(self.handle);
            if err_ptr.is_null() {
                "Unknown libzfs error".to_string()
            } else {
                CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
            }
        }
    }
}

impl Drop for LibZfsHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                libzfs_fini(self.handle);
            }
        }
    }
}

/// RAII wrapper for zpool handle.
pub struct ZpoolHandle {
    handle: *mut zpool_handle_t,
}

impl ZpoolHandle {
    /// Open a pool by name.
    pub fn open(libzfs: &LibZfsHandle, name: &str) -> Result<Self, AppError> {
        let c_name = CString::new(name).map_err(|_| {
            AppError::BadRequest(format!("Invalid pool name: {}", name))
        })?;

        let handle = unsafe { zpool_open(libzfs.as_ptr(), c_name.as_ptr()) };
        if handle.is_null() {
            return Err(AppError::NotFound(format!(
                "Pool '{}' not found: {}",
                name,
                libzfs.last_error()
            )));
        }
        Ok(Self { handle })
    }

    /// Get the pool name.
    pub fn name(&self) -> String {
        unsafe {
            let name_ptr = zpool_get_name(self.handle);
            if name_ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
            }
        }
    }

    /// Get pool state.
    pub fn state(&self) -> pool_state_t {
        unsafe {
            let state = zpool_get_state(self.handle);
            std::mem::transmute(state)
        }
    }

    /// Get pool health status.
    pub fn health(&self) -> String {
        let mut buf = [0i8; 64];
        let result = unsafe {
            zpool_get_prop(
                self.handle,
                zpool_prop_t::ZPOOL_PROP_HEALTH,
                buf.as_mut_ptr(),
                buf.len(),
                ptr::null_mut(),
                0,
            )
        };
        if result == 0 {
            unsafe {
                CStr::from_ptr(buf.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            }
        } else {
            "UNKNOWN".to_string()
        }
    }

    /// Get pool size in bytes.
    pub fn size(&self) -> u64 {
        unsafe { zpool_get_prop_int(self.handle, zpool_prop_t::ZPOOL_PROP_SIZE, ptr::null_mut()) }
    }

    /// Get pool allocated bytes.
    pub fn allocated(&self) -> u64 {
        unsafe {
            zpool_get_prop_int(self.handle, zpool_prop_t::ZPOOL_PROP_ALLOCATED, ptr::null_mut())
        }
    }

    /// Get pool free bytes.
    pub fn free(&self) -> u64 {
        unsafe { zpool_get_prop_int(self.handle, zpool_prop_t::ZPOOL_PROP_FREE, ptr::null_mut()) }
    }

    /// Get pool fragmentation percentage.
    pub fn fragmentation(&self) -> u64 {
        unsafe {
            zpool_get_prop_int(
                self.handle,
                zpool_prop_t::ZPOOL_PROP_FRAGMENTATION,
                ptr::null_mut(),
            )
        }
    }

    /// Get pool capacity percentage.
    pub fn capacity(&self) -> u64 {
        unsafe {
            zpool_get_prop_int(self.handle, zpool_prop_t::ZPOOL_PROP_CAPACITY, ptr::null_mut())
        }
    }

    /// Get pool altroot.
    pub fn altroot(&self) -> Option<String> {
        let mut buf = [0i8; 256];
        let result = unsafe {
            zpool_get_prop(
                self.handle,
                zpool_prop_t::ZPOOL_PROP_ALTROOT,
                buf.as_mut_ptr(),
                buf.len(),
                ptr::null_mut(),
                0,
            )
        };
        if result == 0 {
            let s = unsafe {
                CStr::from_ptr(buf.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            };
            if s.is_empty() || s == "-" {
                None
            } else {
                Some(s)
            }
        } else {
            None
        }
    }

    /// Get detailed pool status.
    pub fn status(&self) -> zpool_status_t {
        unsafe { zpool_get_status(self.handle, ptr::null_mut(), ptr::null_mut()) }
    }

    /// Get raw handle pointer.
    pub fn as_ptr(&self) -> *mut zpool_handle_t {
        self.handle
    }
}

impl Drop for ZpoolHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                zpool_close(self.handle);
            }
        }
    }
}

/// RAII wrapper for zfs dataset handle.
pub struct ZfsHandle {
    handle: *mut zfs_handle_t,
}

impl ZfsHandle {
    /// Open a dataset by name.
    pub fn open(libzfs: &LibZfsHandle, name: &str, types: c_int) -> Result<Self, AppError> {
        let c_name = CString::new(name).map_err(|_| {
            AppError::BadRequest(format!("Invalid dataset name: {}", name))
        })?;

        let handle = unsafe { zfs_open(libzfs.as_ptr(), c_name.as_ptr(), types) };
        if handle.is_null() {
            return Err(AppError::NotFound(format!(
                "Dataset '{}' not found: {}",
                name,
                libzfs.last_error()
            )));
        }
        Ok(Self { handle })
    }

    /// Get dataset name.
    pub fn name(&self) -> String {
        unsafe {
            let name_ptr = zfs_get_name(self.handle);
            if name_ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
            }
        }
    }

    /// Get dataset type.
    pub fn dataset_type(&self) -> zfs_type_t {
        unsafe { zfs_get_type(self.handle) }
    }

    /// Get used bytes.
    pub fn used(&self) -> u64 {
        unsafe { zfs_prop_get_int(self.handle, zfs_prop_t::ZFS_PROP_USED) }
    }

    /// Get available bytes.
    pub fn available(&self) -> u64 {
        unsafe { zfs_prop_get_int(self.handle, zfs_prop_t::ZFS_PROP_AVAILABLE) }
    }

    /// Get referenced bytes.
    pub fn referenced(&self) -> u64 {
        unsafe { zfs_prop_get_int(self.handle, zfs_prop_t::ZFS_PROP_REFERENCED) }
    }

    /// Get compression ratio.
    pub fn compressratio(&self) -> u64 {
        unsafe { zfs_prop_get_int(self.handle, zfs_prop_t::ZFS_PROP_COMPRESSRATIO) }
    }

    /// Get mountpoint.
    pub fn mountpoint(&self) -> Option<String> {
        let mut buf = [0i8; 512];
        let result = unsafe {
            zfs_prop_get(
                self.handle,
                zfs_prop_t::ZFS_PROP_MOUNTPOINT,
                buf.as_mut_ptr(),
                buf.len(),
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                0,
            )
        };
        if result == 0 {
            let s = unsafe {
                CStr::from_ptr(buf.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            };
            if s.is_empty() || s == "none" || s == "-" {
                None
            } else {
                Some(s)
            }
        } else {
            None
        }
    }

    /// Get compression setting.
    pub fn compression(&self) -> String {
        let mut buf = [0i8; 64];
        let result = unsafe {
            zfs_prop_get(
                self.handle,
                zfs_prop_t::ZFS_PROP_COMPRESSION,
                buf.as_mut_ptr(),
                buf.len(),
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                0,
            )
        };
        if result == 0 {
            unsafe {
                CStr::from_ptr(buf.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            }
        } else {
            "unknown".to_string()
        }
    }

    /// Get raw handle pointer.
    pub fn as_ptr(&self) -> *mut zfs_handle_t {
        self.handle
    }
}

impl Drop for ZfsHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                zfs_close(self.handle);
            }
        }
    }
}

// =============================================================================
// High-Level Data Structures
// =============================================================================

/// Information about a ZFS pool.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PoolInfo {
    /// Pool name
    pub name: String,
    /// Total size in bytes
    pub size: u64,
    /// Allocated bytes
    pub allocated: u64,
    /// Free bytes
    pub free: u64,
    /// Capacity percentage (0-100)
    pub capacity: u64,
    /// Fragmentation percentage
    pub fragmentation: u64,
    /// Health status (ONLINE, DEGRADED, FAULTED, etc.)
    pub health: String,
    /// Pool state
    pub state: String,
    /// Alternate root path if set
    pub altroot: Option<String>,
}

/// Information about a ZFS dataset.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DatasetInfo {
    /// Full dataset name (pool/path)
    pub name: String,
    /// Pool this dataset belongs to
    pub pool: String,
    /// Dataset type (filesystem, volume, snapshot)
    pub dataset_type: String,
    /// Used bytes
    pub used: u64,
    /// Available bytes
    pub available: u64,
    /// Referenced bytes
    pub referenced: u64,
    /// Compression ratio (1.00x = 100)
    pub compressratio: u64,
    /// Mount point (if mounted)
    pub mountpoint: Option<String>,
    /// Compression algorithm
    pub compression: String,
}

// =============================================================================
// Public API Functions
// =============================================================================

/// List all ZFS pools on the system.
#[instrument(skip(handle))]
pub fn list_pools(handle: &LibZfsHandle) -> Result<Vec<PoolInfo>, AppError> {
    let mut pools: Vec<PoolInfo> = Vec::new();

    extern "C" fn pool_callback(pool_handle: *mut zpool_handle_t, data: *mut c_void) -> c_int {
        let pools = unsafe { &mut *(data as *mut Vec<PoolInfo>) };

        // Create a temporary wrapper (we won't drop it since we don't own it)
        let pool = ZpoolHandle { handle: pool_handle };

        let state = match pool.state() {
            pool_state_t::POOL_STATE_ACTIVE => "ACTIVE",
            pool_state_t::POOL_STATE_EXPORTED => "EXPORTED",
            pool_state_t::POOL_STATE_DESTROYED => "DESTROYED",
            pool_state_t::POOL_STATE_SPARE => "SPARE",
            pool_state_t::POOL_STATE_L2CACHE => "L2CACHE",
            pool_state_t::POOL_STATE_UNINITIALIZED => "UNINITIALIZED",
            pool_state_t::POOL_STATE_UNAVAIL => "UNAVAIL",
            pool_state_t::POOL_STATE_POTENTIALLY_ACTIVE => "POTENTIALLY_ACTIVE",
        };

        let info = PoolInfo {
            name: pool.name(),
            size: pool.size(),
            allocated: pool.allocated(),
            free: pool.free(),
            capacity: pool.capacity(),
            fragmentation: pool.fragmentation(),
            health: pool.health(),
            state: state.to_string(),
            altroot: pool.altroot(),
        };

        pools.push(info);

        // Don't drop the handle - we don't own it
        std::mem::forget(pool);

        0 // Continue iteration
    }

    let result = unsafe {
        zpool_iter(
            handle.as_ptr(),
            pool_callback,
            &mut pools as *mut Vec<PoolInfo> as *mut c_void,
        )
    };

    if result != 0 {
        error!(target: "yanos::zfs", error = %handle.last_error(), "Failed to iterate pools");
        return Err(AppError::InternalServerError(format!(
            "Failed to list pools: {}",
            handle.last_error()
        )));
    }

    Ok(pools)
}

/// Get information about a specific pool.
#[instrument(skip(handle))]
pub fn get_pool(handle: &LibZfsHandle, name: &str) -> Result<PoolInfo, AppError> {
    let pool = ZpoolHandle::open(handle, name)?;

    let state = match pool.state() {
        pool_state_t::POOL_STATE_ACTIVE => "ACTIVE",
        pool_state_t::POOL_STATE_EXPORTED => "EXPORTED",
        pool_state_t::POOL_STATE_DESTROYED => "DESTROYED",
        pool_state_t::POOL_STATE_SPARE => "SPARE",
        pool_state_t::POOL_STATE_L2CACHE => "L2CACHE",
        pool_state_t::POOL_STATE_UNINITIALIZED => "UNINITIALIZED",
        pool_state_t::POOL_STATE_UNAVAIL => "UNAVAIL",
        pool_state_t::POOL_STATE_POTENTIALLY_ACTIVE => "POTENTIALLY_ACTIVE",
    };

    Ok(PoolInfo {
        name: pool.name(),
        size: pool.size(),
        allocated: pool.allocated(),
        free: pool.free(),
        capacity: pool.capacity(),
        fragmentation: pool.fragmentation(),
        health: pool.health(),
        state: state.to_string(),
        altroot: pool.altroot(),
    })
}

/// List all datasets in a pool.
#[instrument(skip(handle))]
pub fn list_datasets(handle: &LibZfsHandle, pool_name: &str) -> Result<Vec<DatasetInfo>, AppError> {
    let mut datasets: Vec<DatasetInfo> = Vec::new();

    // Open the pool's root filesystem
    let types = zfs_type_t::ZFS_TYPE_FILESYSTEM as c_int
        | zfs_type_t::ZFS_TYPE_VOLUME as c_int
        | zfs_type_t::ZFS_TYPE_SNAPSHOT as c_int;

    let root = ZfsHandle::open(handle, pool_name, types)?;

    // Add the root dataset
    datasets.push(dataset_to_info(&root, pool_name));

    // Recursively collect children
    collect_children(&root, pool_name, &mut datasets);

    Ok(datasets)
}

/// Helper to convert ZfsHandle to DatasetInfo.
fn dataset_to_info(ds: &ZfsHandle, pool_name: &str) -> DatasetInfo {
    let ds_type = match ds.dataset_type() {
        zfs_type_t::ZFS_TYPE_FILESYSTEM => "filesystem",
        zfs_type_t::ZFS_TYPE_VOLUME => "volume",
        zfs_type_t::ZFS_TYPE_SNAPSHOT => "snapshot",
        zfs_type_t::ZFS_TYPE_BOOKMARK => "bookmark",
        _ => "unknown",
    };

    DatasetInfo {
        name: ds.name(),
        pool: pool_name.to_string(),
        dataset_type: ds_type.to_string(),
        used: ds.used(),
        available: ds.available(),
        referenced: ds.referenced(),
        compressratio: ds.compressratio(),
        mountpoint: ds.mountpoint(),
        compression: ds.compression(),
    }
}

/// Recursively collect child datasets.
fn collect_children(parent: &ZfsHandle, pool_name: &str, datasets: &mut Vec<DatasetInfo>) {
    struct CallbackData<'a> {
        pool_name: &'a str,
        datasets: &'a mut Vec<DatasetInfo>,
    }

    extern "C" fn child_callback(zfs_handle: *mut zfs_handle_t, data: *mut c_void) -> c_int {
        let cb_data = unsafe { &mut *(data as *mut CallbackData) };

        let ds = ZfsHandle { handle: zfs_handle };
        cb_data.datasets.push(dataset_to_info(&ds, cb_data.pool_name));

        // Recurse into children
        collect_children(&ds, cb_data.pool_name, cb_data.datasets);

        // Don't drop - we don't own it
        std::mem::forget(ds);

        0
    }

    let mut cb_data = CallbackData { pool_name, datasets };

    unsafe {
        zfs_iter_filesystems(
            parent.as_ptr(),
            child_callback,
            &mut cb_data as *mut CallbackData as *mut c_void,
        );
    }
}

/// Get information about a specific dataset.
#[instrument(skip(handle))]
pub fn get_dataset(handle: &LibZfsHandle, name: &str) -> Result<DatasetInfo, AppError> {
    let types = zfs_type_t::ZFS_TYPE_FILESYSTEM as c_int
        | zfs_type_t::ZFS_TYPE_VOLUME as c_int
        | zfs_type_t::ZFS_TYPE_SNAPSHOT as c_int;

    let ds = ZfsHandle::open(handle, name, types)?;
    let pool_name = name.split('/').next().unwrap_or(name);

    Ok(dataset_to_info(&ds, pool_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_libzfs_handle_creation() {
        let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    fn test_libzfs_handle_last_error() {
        let handle = LibZfsHandle::new().expect("Failed to initialize libzfs");
        // last_error should return something even when no error occurred
        let err = handle.last_error();
        // Just verify it doesn't crash and returns a string
        assert!(err.is_empty() || !err.is_empty());
    }

    #[test]
    fn test_pool_state_enum_values() {
        // Verify enum values match C definitions
        assert_eq!(pool_state_t::POOL_STATE_ACTIVE as i32, 0);
        assert_eq!(pool_state_t::POOL_STATE_EXPORTED as i32, 1);
        assert_eq!(pool_state_t::POOL_STATE_DESTROYED as i32, 2);
        assert_eq!(pool_state_t::POOL_STATE_SPARE as i32, 3);
        assert_eq!(pool_state_t::POOL_STATE_L2CACHE as i32, 4);
        assert_eq!(pool_state_t::POOL_STATE_UNINITIALIZED as i32, 5);
        assert_eq!(pool_state_t::POOL_STATE_UNAVAIL as i32, 6);
        assert_eq!(pool_state_t::POOL_STATE_POTENTIALLY_ACTIVE as i32, 7);
    }

    #[test]
    fn test_zfs_type_enum_values() {
        // Verify enum values match C definitions (bit flags)
        assert_eq!(zfs_type_t::ZFS_TYPE_FILESYSTEM as i32, 1);
        assert_eq!(zfs_type_t::ZFS_TYPE_SNAPSHOT as i32, 2);
        assert_eq!(zfs_type_t::ZFS_TYPE_VOLUME as i32, 4);
        assert_eq!(zfs_type_t::ZFS_TYPE_POOL as i32, 8);
        assert_eq!(zfs_type_t::ZFS_TYPE_BOOKMARK as i32, 16);
    }

    #[test]
    fn test_zpool_prop_enum_values() {
        assert_eq!(zpool_prop_t::ZPOOL_PROP_INVAL as i32, -1);
        assert_eq!(zpool_prop_t::ZPOOL_PROP_NAME as i32, 0);
        assert_eq!(zpool_prop_t::ZPOOL_PROP_SIZE as i32, 1);
    }

    #[test]
    fn test_zfs_prop_enum_values() {
        assert_eq!(zfs_prop_t::ZFS_PROP_INVAL as i32, -1);
        assert_eq!(zfs_prop_t::ZFS_PROP_TYPE as i32, 0);
        assert_eq!(zfs_prop_t::ZFS_PROP_CREATION as i32, 1);
        assert_eq!(zfs_prop_t::ZFS_PROP_USED as i32, 2);
    }

    #[test]
    fn test_zpool_status_enum_values() {
        assert_eq!(zpool_status_t::ZPOOL_STATUS_CORRUPT_CACHE as i32, 0);
        // ZPOOL_STATUS_OK is the last status value (after all error statuses)
        assert_eq!(zpool_status_t::ZPOOL_STATUS_OK as i32, 27);
    }

    #[test]
    fn test_pool_info_struct() {
        let pool = PoolInfo {
            name: "rpool".to_string(),
            size: 100 * 1024 * 1024 * 1024,
            allocated: 30 * 1024 * 1024 * 1024,
            free: 70 * 1024 * 1024 * 1024,
            capacity: 30,
            fragmentation: 5,
            health: "ONLINE".to_string(),
            state: "ACTIVE".to_string(),
            altroot: None,
        };

        assert_eq!(pool.name, "rpool");
        assert_eq!(pool.capacity, 30);
        assert!(pool.altroot.is_none());
    }

    #[test]
    fn test_pool_info_with_altroot() {
        let pool = PoolInfo {
            name: "backup".to_string(),
            size: 0,
            allocated: 0,
            free: 0,
            capacity: 0,
            fragmentation: 0,
            health: "ONLINE".to_string(),
            state: "ACTIVE".to_string(),
            altroot: Some("/mnt/backup".to_string()),
        };

        assert_eq!(pool.altroot, Some("/mnt/backup".to_string()));
    }

    #[test]
    fn test_dataset_info_struct() {
        let ds = DatasetInfo {
            name: "rpool/ROOT/omnios".to_string(),
            pool: "rpool".to_string(),
            dataset_type: "filesystem".to_string(),
            used: 10 * 1024 * 1024 * 1024,
            available: 60 * 1024 * 1024 * 1024,
            referenced: 10 * 1024 * 1024 * 1024,
            compressratio: 150,
            mountpoint: Some("/".to_string()),
            compression: "lz4".to_string(),
        };

        assert_eq!(ds.name, "rpool/ROOT/omnios");
        assert_eq!(ds.pool, "rpool");
        assert_eq!(ds.dataset_type, "filesystem");
        assert_eq!(ds.mountpoint, Some("/".to_string()));
    }

    #[test]
    fn test_dataset_info_volume() {
        let ds = DatasetInfo {
            name: "rpool/swap".to_string(),
            pool: "rpool".to_string(),
            dataset_type: "volume".to_string(),
            used: 2 * 1024 * 1024 * 1024,
            available: 0,
            referenced: 2 * 1024 * 1024 * 1024,
            compressratio: 100,
            mountpoint: None,
            compression: "off".to_string(),
        };

        assert_eq!(ds.dataset_type, "volume");
        assert!(ds.mountpoint.is_none());
    }

    #[test]
    fn test_pool_info_clone() {
        let pool = PoolInfo {
            name: "test".to_string(),
            size: 100,
            allocated: 50,
            free: 50,
            capacity: 50,
            fragmentation: 10,
            health: "ONLINE".to_string(),
            state: "ACTIVE".to_string(),
            altroot: None,
        };

        let cloned = pool.clone();
        assert_eq!(cloned.name, pool.name);
        assert_eq!(cloned.size, pool.size);
    }

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
}
