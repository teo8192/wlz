use std::{error::Error, fmt::Display};

use crate::ffi;
use crate::wrapper::wl::WlDisplay;
use crate::wrapper::wlr::{Allocator, Backend, Renderer};
use crate::wrapper::WrapperError;

#[derive(Debug)]
pub enum WlzError {
    WErr(WrapperError),
}

impl Display for WlzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for WlzError {}

impl From<WrapperError> for WlzError {
    fn from(value: WrapperError) -> Self {
        Self::WErr(value)
    }
}

pub struct WlzServer {
    wl_display: WlDisplay,
    wlr_backend: Backend,
    wlr_renderer: Renderer,
    wlr_allocator: Allocator,
}

impl WlzServer {
    pub fn try_create() -> Result<Self, Box<dyn Error>> {
        let mut wl_display = WlDisplay::try_create()?;
        let wlr_backend = Backend::autocreate(wl_display.get_event_loop())?;
        let mut wlr_renderer = Renderer::autocreate(&wlr_backend)?;

        wlr_renderer.init_wl_display(&mut wl_display)?;

        let wlr_allocator = Allocator::autocreate(&wlr_backend, &wlr_renderer)?;

        unsafe {
            ffi::wlr_compositor_create(wl_display.as_mut_ptr(), 5, wlr_renderer.as_mut_ptr());
            ffi::wlr_subcompositor_create(wl_display.as_mut_ptr());
        }

        Ok(Self {
            wl_display,
            wlr_backend,
            wlr_renderer,
            wlr_allocator,
        })
    }

    pub fn display(&self) -> &WlDisplay {
        &self.wl_display
    }
}
