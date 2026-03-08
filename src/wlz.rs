use std::{error::Error, fmt};

use crate::wrapper::wl::Display;
use crate::wrapper::wlr::{
    Allocator, Backend, Compositor, DataDeviceManager, Renderer, SubCompositor,
};
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

        /* This creates some hands-off wlroots interfaces. The compositor is
         * necessary for clients to allocate surfaces, the subcompositor allows to
         * assign the role of subsurfaces to surfaces and the data device manager
         * handles the clipboard. Each of these wlroots interfaces has room for you
         * to dig your fingers in and play with their behavior if you want. Note that
         * the clients cannot set the selection directly without compositor approval,
         * see the handling of the request_set_selection event below.*/
        Compositor::create(&mut wl_display, 5, &mut wlr_renderer)?;
        SubCompositor::create(&mut wl_display)?;
        DataDeviceManager::create(&mut wl_display)?;

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
