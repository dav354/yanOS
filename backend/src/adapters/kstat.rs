use std::{ffi::CString, io, ptr};

use libc::{c_char, c_int, c_void};

const KSTAT_STRLEN: usize = 31;
const KSTAT_DATA_UINT64: u8 = 4;

#[repr(C)]
pub struct kstat_ctl_t {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct kstat_t {
    _unused: [u8; 0],
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
    pub pad: [u8; 3],
    pub value: kstat_named_value,
}

#[link(name = "kstat")]
unsafe extern "C" {
    fn kstat_open() -> *mut kstat_ctl_t;
    fn kstat_close(kc: *mut kstat_ctl_t) -> c_int;
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
}

pub fn read_arc_size_bytes() -> Result<u64, KstatError> {
    unsafe {
        let kc = kstat_open();
        if kc.is_null() {
            return Err(KstatError::Open(io::Error::last_os_error()));
        }

        let module = CString::new("zfs").map_err(|_| KstatError::Lookup("zfs".to_string()))?;
        let name = CString::new("arcstats").map_err(|_| KstatError::Lookup("arcstats".to_string()))?;
        let metric = CString::new("size").map_err(|_| KstatError::Lookup("size".to_string()))?;

        let ksp = kstat_lookup(kc, module.as_ptr(), 0, name.as_ptr());
        if ksp.is_null() {
            kstat_close(kc);
            return Err(KstatError::Lookup("arcstats".to_string()));
        }

        if kstat_read(kc, ksp, ptr::null_mut()) < 0 {
            let err = io::Error::last_os_error();
            kstat_close(kc);
            return Err(KstatError::Read(err));
        }

        let data_ptr = kstat_data_lookup(ksp, metric.as_ptr());
        if data_ptr.is_null() {
            kstat_close(kc);
            return Err(KstatError::MissingValue);
        }

        let named = &*(data_ptr as *mut kstat_named);
        if named.data_type != KSTAT_DATA_UINT64 {
            kstat_close(kc);
            return Err(KstatError::TypeMismatch(named.data_type));
        }

        let value = named.value.ui64;
        kstat_close(kc);
        Ok(value)
    }
}
