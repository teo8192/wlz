use std::error::Error;
use std::ffi::CStr;

use wlz_macros::{PtrWrapper, cdrop};

use crate::ffi;
use crate::wrapper::WrapperError;

#[derive(PtrWrapper)]
pub struct WlEventLoop(*mut ffi::wl_event_loop);

#[derive(PtrWrapper)]
#[cdrop(ffi::wl_display_destroy)]
pub struct WlDisplay(*mut ffi::wl_display);

unsafe impl Send for WlDisplay {}
unsafe impl Sync for WlDisplay {}

impl WlDisplay {
    pub fn try_create() -> Result<Self, WrapperError> {
        /* The Wayland display is managed by libwayland. It handles accepting
         * clients from the Unix socket, manging Wayland globals, and so on. */
        let wl_display = unsafe { ffi::wl_display_create() };

        if wl_display.is_null() {
            return Err(WrapperError::FailedToCreateDisplay);
        }

        Ok(Self(wl_display))
    }

    pub fn run(&self) {
        unsafe { ffi::wl_display_run(self.0) };
    }

    pub fn terminate(&self) {
        unsafe { ffi::wl_display_terminate(self.0) };
    }

    pub fn add_socket_auto(&self) -> Result<&str, Box<dyn Error>> {
        let socket = unsafe { ffi::wl_display_add_socket_auto(self.0) };

        if socket.is_null() {
            return Err(WrapperError::FailedToAddSocket)?;
        }

        Ok(unsafe { CStr::from_ptr(socket) }.to_str()?)
    }

    pub fn get_event_loop(&self) -> WlEventLoop {
        WlEventLoop(unsafe { ffi::wl_display_get_event_loop(self.0) })
    }
}
