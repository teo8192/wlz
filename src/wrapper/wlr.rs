use std::ptr::null_mut;

use wlz_macros::{PtrWrapper, cdrop};

use super::wl::WlEventLoop;
use super::WrapperError;
use crate::ffi;
use crate::wrapper::wl::WlDisplay;


#[derive(PtrWrapper)]
#[cdrop(ffi::wlr_backend_destroy)]
pub struct Backend(*mut ffi::wlr_backend);

impl Backend {
    pub fn autocreate(mut event_loop: WlEventLoop) -> Result<Self, WrapperError> {
        /* The backend is a wlroots feature which abstracts the underlying input and
         * output hardware. The autocreate option will choose the most suitable
         * backend based on the current environment, such as opening an X11 window
         * if an X11 server is running. */
        let wlr_backend =
            unsafe { ffi::wlr_backend_autocreate(event_loop.as_mut_ptr(), null_mut()) };
        if wlr_backend.is_null() {
            return Err(WrapperError::FailedToCreateBackend);
        }
        Ok(Self(wlr_backend))
    }
}

#[derive(PtrWrapper)]
#[cdrop(ffi::wlr_renderer_destroy)]
pub struct Renderer(*mut ffi::wlr_renderer);

impl Renderer {
    pub fn autocreate(backend: &Backend) -> Result<Self, WrapperError> {
        /* Autocreates a renderer, either Pixman, GLES2 or Vulkan for us. The user
         * can also specify a renderer using the WLR_RENDERER env var.
         * The renderer is responsible for defining the various pixel formats it
         * supports for shared memory, this configures that for clients. */
        let wlr_renderer = unsafe { ffi::wlr_renderer_autocreate(backend.0) };

        if wlr_renderer.is_null() {
            return Err(WrapperError::FailedToCreateRenderer);
        }

        Ok(Self(wlr_renderer))
    }

    pub fn init_wl_display(&mut self, wl_display: &mut WlDisplay) -> Result<(), WrapperError> {
        if unsafe { ffi::wlr_renderer_init_wl_display(self.as_mut_ptr(), wl_display.as_mut_ptr()) }
        {
            Ok(())
        } else {
            Err(WrapperError::FailedToInitializeDisplay)
        }
    }
}

#[derive(PtrWrapper)]
#[cdrop(ffi::wlr_allocator_destroy)]
pub struct Allocator(*mut ffi::wlr_allocator);

impl Allocator {
    pub fn autocreate(
        wlr_backend: &Backend,
        wlr_renderer: &Renderer,
    ) -> Result<Self, WrapperError> {
        /* Autocreates an allocator for us.
         * The allocator is the bridge between the renderer and the backend. It
         * handles the buffer creation, allowing wlroots to render onto the
         * screen */
        let wlr_allocator = unsafe { ffi::wlr_allocator_autocreate(wlr_backend.0, wlr_renderer.0) };

        if wlr_allocator.is_null() {
            return Err(WrapperError::FailedToCreateAllocator);
        }

        Ok(Self(wlr_allocator))
    }
}

#[derive(PtrWrapper)]
pub struct Compositor(*mut ffi::wlr_compositor);

impl Compositor {

}
