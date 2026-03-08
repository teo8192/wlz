use std::{error::Error, fmt};

use crate::wrapper::wl::Display;
use crate::wrapper::wlr::{
    Allocator, Backend, Compositor, DataDeviceManager, OutputLayout, Renderer, SubCompositor,
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
    output_layout: OutputLayout,
    allocator: Allocator,
    renderer: Renderer,
    backend: Backend,
    display: Display,
}

impl WlzServer {
    pub fn try_create() -> Result<Self, Box<dyn Error>> {
        let mut display = Display::try_create()?;
        let mut backend = Backend::autocreate(display.get_event_loop())?;
        let mut renderer = Renderer::autocreate(&mut backend)?;

        renderer.init_wl_display(&mut display)?;

        let allocator = Allocator::autocreate(&mut backend, &mut renderer)?;

        /* This creates some hands-off wlroots interfaces. The compositor is
         * necessary for clients to allocate surfaces, the subcompositor allows to
         * assign the role of subsurfaces to surfaces and the data device manager
         * handles the clipboard. Each of these wlroots interfaces has room for you
         * to dig your fingers in and play with their behavior if you want. Note that
         * the clients cannot set the selection directly without compositor approval,
         * see the handling of the request_set_selection event below.*/
        Compositor::create(&mut display, 5, &mut renderer)?;
        SubCompositor::create(&mut display)?;
        DataDeviceManager::create(&mut display)?;

        /* Creates an output layout, which a wlroots utility for working with an
         * arrangement of screens in a physical layout. */
        let output_layout = OutputLayout::create(&mut display)?;

        Ok(Self {
            output_layout,
            display,
            backend,
            renderer,
            allocator,
        })
    }

    pub fn display(&self) -> &Display {
        &self.display
    }
}
