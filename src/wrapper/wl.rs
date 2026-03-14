use std::error::Error;
use std::ffi::CStr;
use std::marker::{PhantomData, PhantomPinned};
use std::pin::Pin;
use std::ptr::{null_mut, NonNull};

use pin_project::pin_project;
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

/// Reimplementation of `ffi::we_listener`
#[c_ptr(ffi::wl_listener)]
#[repr(C)]
#[pin_project]
pub struct Listener<T = ()> {
    #[pin]
    link: List,
    notify: ffi::wl_notify_func_t,
    _data: PhantomData<T>,
}

type ListenerCallback =
    unsafe extern "C" fn(listener: *mut ffi::wl_listener, data: *mut ::std::os::raw::c_void);

impl<T> Listener<T> {
    pub fn new(notify: ListenerCallback) -> Self {
        Self {
            link: List::empty(),
            notify: Some(notify),
            _data: PhantomData,
        }
    }

    pub fn init(self: Pin<&mut Self>, notify: ListenerCallback) {
        let this = self.project();
        *this.notify = Some(notify);
        this.link.init();
    }

    pub fn empty() -> Self {
        Self {
            link: List::empty(),
            notify: None,
            _data: PhantomData,
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
    pub fn init(self: Pin<&mut Self>) {
        unsafe { ffi::wl_list_init(self.get_unchecked_mut().as_ptr()) };
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

    pub fn insert(self: Pin<&mut Self>, other: Pin<&mut Self>) {
        unsafe {
            ffi::wl_list_insert(
                self.get_unchecked_mut().as_ptr(),
                other.get_unchecked_mut().as_ptr(),
            )
        };
    }
}

#[c_ptr(ffi::wl_signal)]
#[repr(C)]
#[pin_project]
pub struct Signal<T = ()> {
    #[pin]
    listener_list: List,
    _data: PhantomData<T>,
}

pub trait IsUnit {}
impl IsUnit for () {}

impl<T> Signal<T> {
    /// Add the specified listener to this signal.
    ///
    /// # Parameters
    /// - `signal` The signal that will emit events to the listener
    /// - `listener` The listener to add
    ///
    /// # See also
    /// `wl_signal`
    pub fn add(self: Pin<&mut Self>, listener: Pin<&mut Listener<T>>) {
        unsafe {
            ffi::wl_signal_add(
                self.get_unchecked_mut().as_ptr(),
                listener.get_unchecked_mut().as_ptr(),
            )
        };
    }

    pub fn emit(self: Pin<&mut Self>)
    where
        T: IsUnit,
    {
        unsafe { ffi::wl_signal_emit_mutable(self.get_unchecked_mut().as_ptr(), null_mut()) };
    }

    pub fn emit_arg(self: Pin<&mut Self>, data: &mut T) {
        let ptr = data as *mut T;
        unsafe {
            ffi::wl_signal_emit_mutable(
                self.get_unchecked_mut().as_ptr(),
                ptr as *mut std::os::raw::c_void,
            )
        };
    }

    pub fn init(self: Pin<&mut Self>) {
        self.project().listener_list.init();
    }

    pub fn empty() -> Self {
        Self {
            listener_list: List::empty(),
            _data: PhantomData,
        }
    }

    /// # Safety
    ///
    /// This function is unsafe. You must guarantee that the data you return
    /// will not move so long as the argument value does not move (for example,
    /// because it is one of the fields of that value), and also that you do
    /// not move out of the argument you receive to the interior function.
    pub unsafe fn get_event<U, F>(obj: Pin<&U>, func: F) -> Pin<&Self>
    where
        F: Fn(&U) -> &ffi::wl_signal,
    {
        unsafe {
            obj.map_unchecked(|v| {
                (func(v) as *const ffi::wl_signal as *const Self)
                    .as_ref()
                    .unwrap()
            })
        }
    }

    /// # Safety
    ///
    /// This function is unsafe. You must guarantee that the data you return
    /// will not move so long as the argument value does not move (for example,
    /// because it is one of the fields of that value), and also that you do
    /// not move out of the argument you receive to the interior function.
    pub unsafe fn get_event_mut<U, F>(obj: Pin<&mut U>, func: F) -> Pin<&mut Self>
    where
        F: Fn(&mut U) -> &mut ffi::wl_signal,
    {
        unsafe {
            obj.map_unchecked_mut(|v| {
                (func(v) as *mut ffi::wl_signal as *mut Self)
                    .as_mut()
                    .unwrap()
            })
        }
    }
}
