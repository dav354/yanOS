//! Kstat FFI bindings for illumos/OmniOS.
//!
//! This module provides safe wrappers around the illumos kstat(3KSTAT) library
//! for reading kernel statistics. Used primarily for metrics collection:
//! - CPU utilization (via cpu:*:sys kstats)
//! - Memory usage (via unix:0:system_pages)
//! - ZFS ARC size (via zfs:0:arcstats)
//! - Network statistics (via link kstats)

use std::{ffi::CStr, ffi::CString, io, ptr};

use libc::{c_char, c_int, c_uchar, c_uint, c_void};

/// Maximum length of kstat name fields (mirrors KSTAT_STRLEN in sys/kstat.h)
const KSTAT_STRLEN: usize = 31;

/// Kstat data type constants from sys/kstat.h
const KSTAT_DATA_UINT32: u8 = 2;
const KSTAT_DATA_UINT64: u8 = 4;
const KSTAT_TYPE_NAMED: c_uchar = 1;

/// FFI representation of illumos kstat_t structure.
/// Layout must exactly match <sys/kstat.h> for safe FFI access.
///
/// IMPORTANT: Field order and padding are critical. Verify against:
/// `echo '#include <sys/kstat.h>' | cc -E - | grep -A50 'struct kstat'`
#[repr(C)]
pub struct kstat_t {
    pub ks_crtime: i64,                   // hrtime_t - creation time
    pub ks_next: *mut kstat_t,            // next in chain
    pub ks_kid: c_int,                    // unique ID
    pub ks_module: [c_char; KSTAT_STRLEN], // provider module name
    pub ks_resv: c_uchar,                 // reserved
    pub ks_instance: c_int,               // provider instance
    pub ks_name: [c_char; KSTAT_STRLEN],  // kstat name
    pub ks_type: c_uchar,                 // kstat data type
    pub ks_class: [c_char; KSTAT_STRLEN], // kstat class
    pub ks_flags: c_uchar,                // kstat flags
    pub ks_data: *mut c_void,             // pointer to kstat data
    pub ks_ndata: c_uint,                 // number of data records
    pub ks_data_size: usize,              // size of kstat data section
    pub ks_snaptime: i64,                 // hrtime_t - time of last snapshot
    // Private fields - not used but needed for correct struct size
    pub ks_lock: *mut c_void,
    pub ks_update: *mut c_void,
    pub ks_private: *mut c_void,
    pub ks_snapshot: *mut c_void,
    pub ks_lock_class: c_uchar,
}

#[repr(C)]
pub struct kstat_ctl_t {
    pub kc_chain_id: c_int,
    pub kc_chain: *mut kstat_t,
    pub kc_kd: c_int,
}

#[repr(C)]
pub union kstat_named_value {
    pub c: [c_char; 16],
    pub i32: i32,
    pub ui32: u32,
    pub i64: i64,
    pub ui64: u64,
}

#[repr(C)]
pub struct kstat_named {
    pub name: [c_char; KSTAT_STRLEN],
    pub data_type: u8,
    pub value: kstat_named_value,
}

#[link(name = "kstat")]
unsafe extern "C" {
    fn kstat_open() -> *mut kstat_ctl_t;
    #[allow(dead_code)]
    fn kstat_close(kc: *mut kstat_ctl_t) -> c_int;
    fn kstat_chain_update(kc: *mut kstat_ctl_t) -> c_int;
    fn kstat_lookup(
        kc: *mut kstat_ctl_t,
        module: *const c_char,
        instance: c_int,
        name: *const c_char,
    ) -> *mut kstat_t;
    fn kstat_read(kc: *mut kstat_ctl_t, ksp: *mut kstat_t, data: *mut c_void) -> c_int;
    fn kstat_data_lookup(ksp: *mut kstat_t, name: *const c_char) -> *mut c_void;
}

#[derive(Debug)]
pub enum KstatError {
    Open(io::Error),
    Lookup(String),
    Read(io::Error),
    MissingValue,
    TypeMismatch(u8),
    Io(io::Error),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuRawTicks {
    pub idle: u64,
    pub user: u64,
    pub kernel: u64,
}

pub struct KstatReader {
    kc: *mut kstat_ctl_t,
}

unsafe impl Send for KstatReader {}

impl KstatReader {
    fn to_cstring(value: &str) -> Result<CString, KstatError> {
        CString::new(value).map_err(|_| KstatError::Lookup(value.to_string()))
    }

    pub fn new() -> Result<Self, KstatError> {
        let kc = unsafe { kstat_open() };
        if kc.is_null() {
            return Err(KstatError::Open(io::Error::last_os_error()));
        }
        Ok(Self { kc })
    }

    pub fn update(&mut self) -> Result<bool, KstatError> {
        let kid = unsafe { kstat_chain_update(self.kc) };
        if kid < 0 {
            return Err(KstatError::Io(io::Error::last_os_error()));
        }
        Ok(kid != 0) // true if chain changed
    }

    pub fn get_named(
        &mut self,
        module: &str,
        instance: i32,
        name: &str,
        statistic: &str,
    ) -> Result<u64, KstatError> {
        unsafe {
            let c_module = Self::to_cstring(module)?;
            let c_name = Self::to_cstring(name)?;
            let c_stat = Self::to_cstring(statistic)?;

            let ksp = kstat_lookup(self.kc, c_module.as_ptr(), instance, c_name.as_ptr());
            if ksp.is_null() {
                return Err(KstatError::Lookup(format!("{}:{}:{}", module, instance, name)));
            }

            if kstat_read(self.kc, ksp, ptr::null_mut()) < 0 {
                return Err(KstatError::Read(io::Error::last_os_error()));
            }

            let data_ptr = kstat_data_lookup(ksp, c_stat.as_ptr());
            if data_ptr.is_null() {
                return Err(KstatError::MissingValue);
            }

            let named = &*(data_ptr as *mut kstat_named);
            match named.data_type {
                KSTAT_DATA_UINT64 => Ok(named.value.ui64),
                KSTAT_DATA_UINT32 => Ok(named.value.ui32 as u64),
                other => Err(KstatError::TypeMismatch(other)),
            }
        }
    }

    /// Iterates over all kstats matching `module`, assuming they are NAMED type,
    /// and extracts the `field` value (summed).
    pub fn sum_named(&mut self, module: &str, field: &str) -> u64 {
        let mut sum = 0;
        unsafe {
            let mut ksp = (*self.kc).kc_chain;
            let target_module = match Self::to_cstring(module) {
                Ok(v) => v,
                Err(_) => return 0,
            };
            let target_field = match Self::to_cstring(field) {
                Ok(v) => v,
                Err(_) => return 0,
            };

            while !ksp.is_null() {
                let ks = &*ksp;
                ksp = ks.ks_next; // Advance

                if ks.ks_type != KSTAT_TYPE_NAMED {
                    continue;
                }

                // Check module match
                let mod_name = CStr::from_ptr(ks.ks_module.as_ptr());
                if mod_name.to_bytes() != target_module.as_bytes() {
                    continue;
                }

                // Read data
                if kstat_read(self.kc, ks as *const _ as *mut _, ptr::null_mut()) < 0 {
                    continue;
                }

                // Find field manually in the data array
                // kstat_data_lookup works on specific kstat, reusing it is cleaner
                // Note: passing mutable pointer to ksp which is already iterating is fine because lookup doesn't mutate chain position
                let data_ptr = kstat_data_lookup(ks as *const kstat_t as *mut kstat_t, target_field.as_ptr());
                if !data_ptr.is_null() {
                    let named = &*(data_ptr as *mut kstat_named);
                    match named.data_type {
                        KSTAT_DATA_UINT64 => sum += named.value.ui64,
                        KSTAT_DATA_UINT32 => sum += named.value.ui32 as u64,
                        _ => {}
                    }
                }
            }
        }
        sum
    }

    /// Sum a named field across all NAMED kstats, optionally filtering by class prefix.
    pub fn sum_field_any(&mut self, field: &str, class_prefix: Option<&str>) -> u64 {
        let mut sum = 0;
        unsafe {
            let target_field = match Self::to_cstring(field) {
                Ok(v) => v,
                Err(_) => return 0,
            };
            let mut ksp = (*self.kc).kc_chain;

            while !ksp.is_null() {
                let ks = &*ksp;
                ksp = ks.ks_next;

                if ks.ks_type != KSTAT_TYPE_NAMED {
                    continue;
                }

                if let Some(prefix) = class_prefix {
                    let cls = CStr::from_ptr(ks.ks_class.as_ptr());
                    if !cls.to_string_lossy().starts_with(prefix) {
                        continue;
                    }
                }

                if kstat_read(self.kc, ks as *const _ as *mut _, ptr::null_mut()) < 0 {
                    continue;
                }

                let data_ptr =
                    kstat_data_lookup(ks as *const kstat_t as *mut kstat_t, target_field.as_ptr());
                if !data_ptr.is_null() {
                    let named = &*(data_ptr as *mut kstat_named);
                    match named.data_type {
                        KSTAT_DATA_UINT64 => sum += named.value.ui64,
                        KSTAT_DATA_UINT32 => sum += named.value.ui32 as u64,
                        _ => {}
                    }
                }
            }
        }
        sum
    }

    pub fn get_aggregate_cpu_ticks(&mut self) -> CpuRawTicks {
        // Aggregate by summing all per-instance ticks
        let per_instance = self.get_cpu_ticks_by_instance();
        let mut ticks = CpuRawTicks::default();
        for (_, t) in per_instance {
            ticks.idle += t.idle;
            ticks.user += t.user;
            ticks.kernel += t.kernel;
        }
        ticks
    }

    pub fn get_cpu_ticks_by_instance(&mut self) -> Vec<(i32, CpuRawTicks)> {
        let mut out = Vec::new();
        unsafe {
            let mut ksp = (*self.kc).kc_chain;
            // Use cpu:*:sys kstats (KSTAT_TYPE_NAMED) instead of cpu_stat (KSTAT_TYPE_RAW)
            let module_name = match Self::to_cstring("cpu") {
                Ok(v) => v,
                Err(_) => return out,
            };
            let kstat_name = match Self::to_cstring("sys") {
                Ok(v) => v,
                Err(_) => return out,
            };
            let f_idle = match Self::to_cstring("cpu_ticks_idle") {
                Ok(v) => v,
                Err(_) => return out,
            };
            let f_user = match Self::to_cstring("cpu_ticks_user") {
                Ok(v) => v,
                Err(_) => return out,
            };
            let f_kernel = match Self::to_cstring("cpu_ticks_kernel") {
                Ok(v) => v,
                Err(_) => return out,
            };

            while !ksp.is_null() {
                let ks = &*ksp;
                ksp = ks.ks_next;

                if ks.ks_type != KSTAT_TYPE_NAMED {
                    continue;
                }

                let mod_cstr = CStr::from_ptr(ks.ks_module.as_ptr());
                if mod_cstr.to_bytes() != module_name.as_bytes() {
                    continue;
                }

                let name_cstr = CStr::from_ptr(ks.ks_name.as_ptr());
                if name_cstr.to_bytes() != kstat_name.as_bytes() {
                    continue;
                }

                if kstat_read(self.kc, ks as *const _ as *mut _, ptr::null_mut()) < 0 {
                    continue;
                }

                let lookup = |field: &CString| -> u64 {
                    let ptr = kstat_data_lookup(ks as *const _ as *mut _, field.as_ptr());
                    if ptr.is_null() {
                        return 0;
                    }
                    let n = &*(ptr as *mut kstat_named);
                    match n.data_type {
                        KSTAT_DATA_UINT64 => n.value.ui64,
                        KSTAT_DATA_UINT32 => n.value.ui32 as u64,
                        _ => 0,
                    }
                };

                let ticks = CpuRawTicks {
                    idle: lookup(&f_idle),
                    user: lookup(&f_user),
                    kernel: lookup(&f_kernel),
                };

                if ticks.idle == 0 && ticks.user == 0 && ticks.kernel == 0 {
                    continue;
                }

                out.push((ks.ks_instance, ticks));
            }
        }
        out
    }

    // Helper to get total ZFS ARC size
    pub fn get_arc_size(&mut self) -> u64 {
        self.get_named("zfs", 0, "arcstats", "size").unwrap_or(0)
    }

    // Helper to get memory pages
    pub fn get_memory_pages(&mut self) -> (u64, u64) {
        let total = self.get_named("unix", 0, "system_pages", "physmem").unwrap_or(0);
        let free = self.get_named("unix", 0, "system_pages", "freemem").unwrap_or(0);
        (total, free)
    }
}

impl Drop for KstatReader {
    fn drop(&mut self) {
        unsafe {
            let _ = kstat_close(self.kc);
        }
    }
}
