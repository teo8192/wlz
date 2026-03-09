#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
#![allow(unnecessary_transmutes)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::ptr_offset_with_cast)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Add the specified listener to this signal.
///
/// # Parameters
/// - `signal` The signal that will emit events to the listener
/// - `listener` The listener to add
///
/// # See also
/// `wl_signal`
pub unsafe fn wl_signal_add(signal: *mut wl_signal, listener: *mut wl_listener) {
    // reimplement the same function that is defined in the <wayland-server-core.h> header (i wish
    // bindgen would just do this, but oh well)
    unsafe {
        wl_list_insert(
            (*signal).listener_list.prev,
            &mut (*listener).link as *mut wl_list,
        )
    };
}
