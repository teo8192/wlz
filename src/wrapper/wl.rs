use std::error::Error;
use std::ffi::CStr;
use std::marker::PhantomPinned;
use std::ptr::{null_mut, NonNull};

use wlz_macros::{c_drop, c_ptr, PtrWrapper};

use crate::ffi;
use crate::wrapper::WrapperError;

#[derive(PtrWrapper)]
pub struct EventLoop(NonNull<ffi::wl_event_loop>);

#[derive(PtrWrapper)]
#[c_drop(ffi::wl_display_destroy)]
pub struct Display(NonNull<ffi::wl_display>);

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

/// Reimplementation of `ffi::wl_listener`
#[c_ptr(ffi::wl_listener)]
#[repr(C)]
pub struct Listener {
    link: List,
    notify: ffi::wl_notify_func_t,
}

type ListenerCallback =
    unsafe extern "C" fn(listener: *mut ffi::wl_listener, data: *mut ::std::os::raw::c_void);

impl Listener {
    pub fn new(notify: ListenerCallback) -> Self {
        Self {
            link: List::empty(),
            notify: Some(notify),
        }
    }

    pub fn init(&mut self, notify: ListenerCallback) {
        self.notify = Some(notify);
        self.link.init();
    }

    pub fn empty() -> Self {
        Self {
            link: List::empty(),
            notify: None,
        }
    }
}

#[c_ptr(ffi::wl_list)]
#[repr(C)]
/// Wrapper around `ffi::wl_list`
pub struct List {
    list: ffi::wl_list,
    _pin: PhantomPinned,
}

impl Drop for List {
    fn drop(&mut self) {
        if self.list.next.is_null() != self.list.prev.is_null() {
            panic!("Trying to drop partially initialised list!");
        }
        if !self.list.next.is_null() {
            unsafe { ffi::wl_list_remove(self.as_ptr()) };
        } else {
            panic!("Trying to drop uninitialized list!");
        }
    }
}

impl List {
    pub fn init(&mut self) {
        unsafe { ffi::wl_list_init(self.as_ptr()) };
    }

    pub fn empty() -> Self {
        Self {
            list: ffi::wl_list {
                prev: null_mut(),
                next: null_mut(),
            },
            _pin: PhantomPinned,
        }
    }

    pub fn insert(&mut self, other: &mut Self) {
        unsafe { ffi::wl_list_insert(self.as_ptr(), other.as_ptr()) };
    }
}

#[c_ptr(ffi::wl_signal)]
#[repr(C)]
pub struct Signal {
    listener_list: List,
}

impl Signal {
    pub fn add(&mut self, listener: &mut Listener) {
        unsafe { ffi::wl_signal_add(self.as_ptr(), listener.as_ptr()) };
    }

    pub fn emit(&mut self) {
        unsafe { ffi::wl_signal_emit_mutable(self.as_ptr(), null_mut()) };
    }

    pub fn emit_arg<T>(&mut self, data: &mut T) {
        let ptr = data as *mut T;
        unsafe { ffi::wl_signal_emit_mutable(self.as_ptr(), ptr as *mut std::os::raw::c_void) };
    }

    pub fn init(&mut self) {
        self.listener_list.init();
    }

    pub fn empty() -> Self {
        Self {
            listener_list: List::empty(),
        }
    }
}
