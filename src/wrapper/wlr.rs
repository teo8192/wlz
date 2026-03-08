use std::ptr::{null_mut, NonNull};

use wlz_macros::{cdrop, PtrWrapper};

use super::wl::EventLoop;
use super::WrapperError;
use crate::ffi;
use crate::wrapper::wl::Display;

#[derive(PtrWrapper)]
#[cdrop(ffi::wlr_backend_destroy)]
pub struct Backend(NonNull<ffi::wlr_backend>);

impl Backend {
    pub fn autocreate(event_loop: EventLoop) -> Result<Self, WrapperError> {
        /* The backend is a wlroots feature which abstracts the underlying input and
         * output hardware. The autocreate option will choose the most suitable
         * backend based on the current environment, such as opening an X11 window
         * if an X11 server is running. */
        NonNull::new(unsafe { ffi::wlr_backend_autocreate(event_loop.as_ptr(), null_mut()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateBackend)
    }
}

#[derive(PtrWrapper)]
#[cdrop(ffi::wlr_renderer_destroy)]
pub struct Renderer(NonNull<ffi::wlr_renderer>);

impl Renderer {
    pub fn autocreate(backend: &mut Backend) -> Result<Self, WrapperError> {
        /* Autocreates a renderer, either Pixman, GLES2 or Vulkan for us. The user
         * can also specify a renderer using the WLR_RENDERER env var.
         * The renderer is responsible for defining the various pixel formats it
         * supports for shared memory, this configures that for clients. */
        NonNull::new(unsafe { ffi::wlr_renderer_autocreate(backend.as_ptr()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateRenderer)
    }

    pub fn init_wl_display(&mut self, wl_display: &mut Display) -> Result<(), WrapperError> {
        if unsafe { ffi::wlr_renderer_init_wl_display(self.as_ptr(), wl_display.as_ptr()) } {
            Ok(())
        } else {
            Err(WrapperError::FailedToInitializeDisplay)
        }
    }
}

#[derive(PtrWrapper)]
#[cdrop(ffi::wlr_allocator_destroy)]
pub struct Allocator(NonNull<ffi::wlr_allocator>);

impl Allocator {
    pub fn autocreate(
        wlr_backend: &mut Backend,
        wlr_renderer: &mut Renderer,
    ) -> Result<Self, WrapperError> {
        /* Autocreates an allocator for us.
         * The allocator is the bridge between the renderer and the backend. It
         * handles the buffer creation, allowing wlroots to render onto the
         * screen */
        NonNull::new(unsafe {
            ffi::wlr_allocator_autocreate(wlr_backend.as_ptr(), wlr_renderer.as_ptr())
        })
        .map(Self)
        .ok_or(WrapperError::FailedToCreateAllocator)
    }
}

#[derive(PtrWrapper)]
pub struct Compositor(NonNull<ffi::wlr_compositor>);

impl Compositor {
    pub fn create(
        wl_display: &mut Display,
        wlr_renderer: &mut Renderer,
    ) -> Result<Self, WrapperError> {
        NonNull::new(unsafe {
            ffi::wlr_compositor_create(wl_display.as_ptr(), 5, wlr_renderer.as_ptr())
        })
        .map(|ptr| Self(ptr))
        .ok_or(WrapperError::FailedToCreateCompositor)
    }
}
