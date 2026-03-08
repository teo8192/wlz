use std::error::Error;
use std::ffi::CStr;
use std::mem;
use std::ptr::NonNull;

use wlz_macros::{cdrop, PtrWrapper};

use crate::ffi;
use crate::wrapper::WrapperError;

#[derive(PtrWrapper)]
pub struct EventLoop(NonNull<ffi::wl_event_loop>);

#[derive(PtrWrapper)]
#[cdrop(ffi::wl_display_destroy)]
pub struct Display(NonNull<ffi::wl_display>);

unsafe impl Send for Display {}
unsafe impl Sync for Display {}

impl Display {
    pub fn try_create() -> Result<Self, WrapperError> {
        /* The Wayland display is managed by libwayland. It handles accepting
         * clients from the Unix socket, manging Wayland globals, and so on. */
        NonNull::new(unsafe { ffi::wl_display_create() })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateDisplay)
    }

    pub fn run(&mut self) {
        unsafe { ffi::wl_display_run(self.as_ptr()) };
    }

    pub fn terminate(&mut self) {
        unsafe { ffi::wl_display_terminate(self.as_ptr()) };
    }

    pub fn add_socket_auto(&mut self) -> Result<&str, Box<dyn Error>> {
        let socket = unsafe { ffi::wl_display_add_socket_auto(self.as_ptr()) };

        if socket.is_null() {
            return Err(WrapperError::FailedToAddSocket)?;
        }

        Ok(unsafe { CStr::from_ptr(socket) }.to_str()?)
    }

    pub fn get_event_loop(&mut self) -> EventLoop {
        EventLoop(NonNull::new(unsafe { ffi::wl_display_get_event_loop(self.as_ptr()) }).unwrap())
    }
}

pub struct Listener(pub ffi::wl_listener);

impl Listener {
    /// Safety: the pointer must be valid for mutable access
    pub unsafe fn from_ptr<'a>(ptr: NonNull<ffi::wl_listener>) -> &'a mut Listener {
        let ptr = ptr.as_ptr() as *mut Listener;
        unsafe { &mut *ptr }
    }
}

pub struct List(ffi::wl_list);

impl List {
    pub fn new() -> Self {
        let mut list = unsafe { mem::zeroed() };
        unsafe { ffi::wl_list_init(&mut list as *mut ffi::wl_list) };

        Self(list)
    }
}

pub struct Signal(ffi::wl_signal);
