use std::{error::Error, fmt};

use crate::ffi;
use crate::wrapper::wl::Display;
use crate::wrapper::wlr::{Allocator, Backend, Renderer};
use crate::wrapper::WrapperError;

#[derive(Debug)]
pub enum WlzError {
    WErr(WrapperError),
}

impl fmt::Display for WlzError {
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
    // field order is important, they are dropped in the order they are declared
    wlr_allocator: Allocator,
    wlr_renderer: Renderer,
    wlr_backend: Backend,
    wl_display: Display,
}

impl WlzServer {
    pub fn try_create() -> Result<Self, Box<dyn Error>> {
        let mut wl_display = Display::try_create()?;
        let mut wlr_backend = Backend::autocreate(wl_display.get_event_loop())?;
        let mut wlr_renderer = Renderer::autocreate(&mut wlr_backend)?;

        wlr_renderer.init_wl_display(&mut wl_display)?;

        let wlr_allocator = Allocator::autocreate(&mut wlr_backend, &mut wlr_renderer)?;



        unsafe {
            ffi::wlr_compositor_create(wl_display.as_ptr(), 5, wlr_renderer.as_ptr());
            ffi::wlr_subcompositor_create(wl_display.as_ptr());
        }

        Ok(Self {
            wl_display,
            wlr_backend,
            wlr_renderer,
            wlr_allocator,
        })
    }

    pub fn display(&self) -> &Display {
        &self.wl_display
    }
}
