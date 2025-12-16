//! Network change watcher using illumos sysevent(7D).
//!
//! Listens for link state changes published by the kernel and forwards them
//! to the `EventBus` so the UI can refresh without polling.
//!
//! This watcher is illumos-specific and requires libsysevent and libnvpair.
//! On other operating systems, this will fail gracefully.

use std::ffi::{c_char, c_int, CStr, CString};
use std::ptr;
use std::sync::OnceLock;
use std::thread::JoinHandle;
use std::time::Duration;

use tracing::{error, info, warn};

use crate::error::AppError;
use crate::events::{EventBus, ExternalEvent};

/// Global EventBus reference for the sysevent callback.
/// This is necessary because libsysevent's callback signature doesn't support a cookie parameter.
static EVENT_BUS: OnceLock<EventBus> = OnceLock::new();

// FFI types for illumos sysevent(7D)
#[repr(C)]
struct SyseventHandle {
    _priv: [u8; 0],
}

#[repr(C)]
struct Sysevent {
    _priv: [u8; 0],
}

#[repr(C)]
struct NvList {
    _priv: [u8; 0],
}

/// Callback function type for sysevent_bind_handle.
/// Note: The actual signature is `void (*)(sysevent_t *)` - no cookie parameter.
type SyseventCallback = Option<unsafe extern "C" fn(ev: *mut Sysevent)>;

#[link(name = "sysevent")]
unsafe extern "C" {
    /// Bind a handler function to receive system events.
    /// Returns an opaque handle, or NULL on failure.
    fn sysevent_bind_handle(handler: SyseventCallback) -> *mut SyseventHandle;

    /// Unbind and release the sysevent handle.
    #[allow(dead_code)] // Will be used for graceful shutdown later
    fn sysevent_unbind_handle(handle: *mut SyseventHandle);

    /// Subscribe to events of a specific class.
    /// subclass_list is a NULL-terminated array of subclass names, or NULL for all subclasses.
    /// num_subclasses is the count (0 means subscribe to all).
    fn sysevent_subscribe_event(
        handle: *mut SyseventHandle,
        event_class: *const c_char,
        subclass_list: *const *const c_char,
        num_subclasses: c_int,
    ) -> c_int;

    /// Get the class name from an event.
    /// Returns the class name as a string (do not free).
    fn sysevent_get_class_name(ev: *mut Sysevent) -> *mut c_char;

    /// Get the subclass name from an event.
    /// Returns the subclass name as a string (do not free).
    fn sysevent_get_subclass_name(ev: *mut Sysevent) -> *mut c_char;

    /// Get the attribute list from an event.
    /// On success, nvlist is set to the attribute list (caller must free with nvlist_free).
    fn sysevent_get_attr_list(ev: *mut Sysevent, nvlist: *mut *mut NvList) -> c_int;
}

#[link(name = "nvpair")]
unsafe extern "C" {
    fn nvlist_lookup_string(
        nvl: *const NvList,
        name: *const c_char,
        val: *mut *mut c_char,
    ) -> c_int;
    fn nvlist_free(nvl: *mut NvList);
}

/// Sysevent callback handler.
/// Called by libsysevent when a network event is received.
unsafe extern "C" fn sysevent_handler(ev: *mut Sysevent) {
    if ev.is_null() {
        return;
    }

    let Some(bus) = EVENT_BUS.get() else {
        return;
    };

    // Get event class
    let class_ptr = unsafe { sysevent_get_class_name(ev) };
    let class = if !class_ptr.is_null() {
        unsafe { CStr::from_ptr(class_ptr) }
            .to_string_lossy()
            .into_owned()
    } else {
        String::new()
    };

    // Get event subclass
    let subclass_ptr = unsafe { sysevent_get_subclass_name(ev) };
    let subclass = if !subclass_ptr.is_null() {
        unsafe { CStr::from_ptr(subclass_ptr) }
            .to_string_lossy()
            .into_owned()
    } else {
        String::new()
    };

    // Extract link name if present in attributes
    let mut attr: *mut NvList = ptr::null_mut();
    let mut link_name: Option<String> = None;
    if unsafe { sysevent_get_attr_list(ev, &mut attr) } == 0 && !attr.is_null() {
        let mut name_ptr: *mut c_char = ptr::null_mut();
        for key in ["LINK", "LINKNAME", "LIFNAME", "DEVNAME"] {
            let key_c = CString::new(key).unwrap();
            if unsafe { nvlist_lookup_string(attr, key_c.as_ptr(), &mut name_ptr) } == 0
                && !name_ptr.is_null()
            {
                link_name =
                    Some(unsafe { CStr::from_ptr(name_ptr) }.to_string_lossy().into_owned());
                break;
            }
        }
        unsafe { nvlist_free(attr) };
    }

    // Log the event for debugging
    info!(
        target: "yanos::watcher",
        class = %class,
        subclass = %subclass,
        link = ?link_name,
        "Received network sysevent"
    );

    // Publish appropriate event to the bus
    match (class.as_str(), subclass.as_str()) {
        (_, "ESC_LINK_UP") => {
            if let Some(name) = link_name {
                bus.publish(ExternalEvent::LinkUp { name });
            } else {
                bus.publish(ExternalEvent::ConfigChanged {
                    path: std::path::PathBuf::from("/dev/sysevent/link_up"),
                });
            }
        }
        (_, "ESC_LINK_DOWN") => {
            if let Some(name) = link_name {
                bus.publish(ExternalEvent::LinkDown { name });
            } else {
                bus.publish(ExternalEvent::ConfigChanged {
                    path: std::path::PathBuf::from("/dev/sysevent/link_down"),
                });
            }
        }
        // For other network/IP changes, emit a generic config change to trigger UI refresh
        ("EC_LINK", _) | ("EC_NETWORK", _) | ("EC_IP", _) => {
            bus.publish(ExternalEvent::ConfigChanged {
                path: std::path::PathBuf::from("/dev/sysevent/network"),
            });
        }
        _ => {}
    }
}

/// Start the network sysevent watcher (illumos only).
///
/// This function spawns a thread that listens for kernel network events
/// (link up/down, IP changes) via the sysevent framework and publishes
/// them to the EventBus.
///
/// On non-illumos systems, this will fail with an error about missing libraries.
pub fn start_network_event_watcher(bus: EventBus) -> Result<JoinHandle<()>, AppError> {
    // Store the EventBus globally for the callback
    EVENT_BUS.set(bus).map_err(|_| {
        AppError::ServiceUnavailable("Network event watcher already initialized".to_string())
    })?;

    info!(target: "yanos::watcher", "Starting network sysevent watcher thread");

    let watcher = std::thread::Builder::new()
        .name("network-sysevent-watcher".to_string())
        .spawn(move || {
            info!(target: "yanos::watcher", "Network sysevent watcher thread spawned, initializing sysevent");

            unsafe {
                info!(target: "yanos::watcher", "Binding sysevent handle");
                let handle = sysevent_bind_handle(Some(sysevent_handler));
                if handle.is_null() {
                    error!(target: "yanos::watcher", "Failed to bind sysevent handle - libsysevent may not be available");
                    return;
                }

                // Subscribe to network-related event classes
                // Pass NULL for subclass_list and 0 for num_subclasses to get all subclasses
                let event_classes = [
                    ("EC_LINK", CString::new("EC_LINK").unwrap()),
                    ("EC_NETWORK", CString::new("EC_NETWORK").unwrap()),
                    ("EC_IP", CString::new("EC_IP").unwrap()),
                ];

                for (name, class) in &event_classes {
                    let rc = sysevent_subscribe_event(
                        handle,
                        class.as_ptr(),
                        ptr::null(), // NULL = all subclasses
                        0,           // 0 = all subclasses
                    );
                    if rc != 0 {
                        warn!(
                            target: "yanos::watcher",
                            class = *name,
                            rc,
                            "Failed to subscribe to sysevent class"
                        );
                    } else {
                        info!(target: "yanos::watcher", class = *name, "Subscribed to sysevent class");
                    }
                }

                info!(target: "yanos::watcher", "Network sysevent watcher ready, listening for events");

                // Keep the thread alive - libsysevent delivers events via the callback
                loop {
                    std::thread::sleep(Duration::from_secs(60));
                }
            }
        })
        .map_err(|e| AppError::ServiceUnavailable(format!("Failed to spawn network watcher: {e}")))?;

    info!(target: "yanos::watcher", "Network sysevent watcher thread created");
    Ok(watcher)
}
