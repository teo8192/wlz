use std::ffi::CStr;

use crate::ffi;

pub struct WlDisplay {
    disp: *mut ffi::wl_display,
}

unsafe impl Send for WlDisplay {}
unsafe impl Sync for WlDisplay {}

impl WlDisplay {
    pub fn try_create() -> Option<Self> {
        let disp = unsafe { ffi::wl_display_create() };

        if disp.is_null() {
            return None;
        }

        Some(Self { disp })
    }

    pub fn run(&self) {
        unsafe {
            ffi::wl_display_run(self.disp);
        }
    }

    pub fn terminate(&self) {
        unsafe {
            ffi::wl_display_terminate(self.disp);
        }
    }

    pub fn add_socket_auto(&self) -> Option<&str> {
        let socket = unsafe { ffi::wl_display_add_socket_auto(self.disp) };

        if socket.is_null() {
            return None;
        }

        unsafe { CStr::from_ptr(socket) }.to_str().ok()
    }
}

impl Drop for WlDisplay {
    fn drop(&mut self) {
        unsafe {
            ffi::wl_display_destroy(self.disp);
        }
    }
}
