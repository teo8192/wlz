use std::mem;
use std::pin::Pin;
use std::ptr::NonNull;
use std::{error::Error, fmt};

use wlz_macros::WlListeners;

use crate::wrapper::wl::{Display, List, Listener};
use crate::wrapper::wlr::{
    Allocator, Backend, BackendEvent, Compositor, DataDeviceManager, Output, OutputEvent,
    OutputLayout, OutputState, Renderer, Scene, SceneOutputLayout, SubCompositor,
};
use crate::wrapper::WrapperError;
use crate::{error, ffi};

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

#[derive(WlListeners)]
pub struct WlzServer {
    outputs: List,
    #[listener("new_output", Output)]
    new_output: Listener,
    // field order is important, they are dropped in the order they are declared
    output_layout: OutputLayout,

    scene_layout: SceneOutputLayout,
    scene: Scene,

    allocator: Allocator,
    renderer: Renderer,
    backend: Backend,
    display: Display,
}

impl WlzServer {
    pub fn initialize(self: Pin<&mut Self>) -> Result<(), Box<dyn Error>> {
        // SAFETY: self is pinned
        let this = unsafe { self.get_unchecked_mut() };

        this.display = Display::try_create()?;
        this.backend = Backend::autocreate(this.display.get_event_loop())?;
        this.renderer = Renderer::autocreate(&mut this.backend)?;

        this.renderer.init_wl_display(&mut this.display)?;

        this.allocator = Allocator::autocreate(&mut this.backend, &mut this.renderer)?;

        /* This creates some hands-off wlroots interfaces. The compositor is
         * necessary for clients to allocate surfaces, the subcompositor allows to
         * assign the role of subsurfaces to surfaces and the data device manager
         * handles the clipboard. Each of these wlroots interfaces has room for you
         * to dig your fingers in and play with their behavior if you want. Note that
         * the clients cannot set the selection directly without compositor approval,
         * see the handling of the request_set_selection event below.*/
        Compositor::create(&mut this.display, 5, &mut this.renderer)?;
        SubCompositor::create(&mut this.display)?;
        DataDeviceManager::create(&mut this.display)?;

        /* Creates an output layout, which a wlroots utility for working with an
         * arrangement of screens in a physical layout. */
        this.output_layout = OutputLayout::create(&mut this.display)?;

        /* Configure a listener to be notified when new outputs are available on the
         * backend. */
        this.outputs = List::new();

        this.new_output =
            Self::init_new_output(this.backend.get_event_mut(BackendEvent::NewOutput));

        Ok(())
    }

    /// This event is raised by the backend when a new output (aka a display or
    /// monitor) becomes available.
    pub fn new_output(&mut self, wlr_output: &mut Output) {
        /* Configures the output created by the backend to use our allocator
         * and our renderer. Must be done once, before commiting the output */
        wlr_output.init_renderer(&mut self.allocator, &mut self.renderer);

        /* The output may be disabled, switch it on. */
        let mut state = OutputState::new();
        state.set_enabled(true);

        /* Some backends don't have modes. DRM+KMS does, and we need to set a mode
         * before we can use the output. The mode is a tuple of (width, height,
         * refresh rate), and each monitor supports only a specific set of modes. We
         * just pick the monitor's preferred mode, a more sophisticated compositor
         * would let the user configure it. */
        if let Some(mode) = wlr_output.preferred_mode() {
            state.set_mode(mode)
        }

        /* Atomically applies the new output state. */
        wlr_output.commit_state(&mut state);
        state.finish();

        /* Allocates and configures our state for this output */
        let mut pinned = unsafe { WlzOutput::uninitialized() };
        pinned.as_mut().initialize(self, wlr_output);
        let output = unsafe { pinned.as_mut().get_unchecked_mut() };

        /* Sets up a listener for the frame event. */
        output.frame = WlzOutput::init_frame(wlr_output.get_event_mut(OutputEvent::Frame));

        /* Sets up a listener for the state request event. */
        output.request_state =
            WlzOutput::init_request_state(wlr_output.get_event_mut(OutputEvent::RequestState));

        /* Sets up a listener for the destroy event. */
        output.destroy = WlzOutput::init_destroy(wlr_output.get_event_mut(OutputEvent::Destroy));

        self.outputs.insert(&mut output.link);

        /* Adds this to the output layout. The add_auto function arranges outputs
         * from left-to-right in the order they appear. A more sophisticated
         * compositor would let the user configure the arrangement of outputs in the
         * layout.
         *
         * The output layout utility automatically adds a wl_output global to the
         * display, which Wayland clients can see to find out information about the
         * output (such as DPI, scale factor, manufacturer, etc).
         */
        if let Err(e) = (|| {
            let mut l_output = self.output_layout.add_auto(wlr_output)?;
            let mut scene_output = self.scene.output_create(wlr_output)?;
            self.scene_layout
                .add_output(&mut l_output, &mut scene_output);
            Ok::<(), WrapperError>(())
        })() {
            error!("Failure during adding of scene output: {}", e);
        }

        // forget the memory, it is deallocated when destroy signal is received
        mem::forget(pinned);
    }

    pub fn display(&self) -> &Display {
        &self.display
    }
}

#[derive(WlListeners)]
struct WlzOutput {
    link: List,
    server: NonNull<WlzServer>,
    output: NonNull<Output>,
    #[listener("frame")]
    frame: Listener,
    #[listener("request_state")]
    request_state: Listener,
    #[listener("destroy")]
    destroy: Listener,
}

impl WlzOutput {
    fn initialize(self: Pin<&mut Self>, server: &mut WlzServer, output: &mut Output) {
        // SAFETY: self is pinned
        let this = unsafe { self.get_unchecked_mut() };

        this.server = NonNull::new(server as *mut WlzServer).unwrap();
        this.output = NonNull::new(output as *mut Output).unwrap();
    }

    fn destroy(&mut self) {
        unsafe { ffi::wl_list_remove(self.link.as_ptr()) };
        // create the box to let it go out of scope to drop all stuff in this thing
        drop(unsafe { Box::from_raw(self as *mut Self) });
    }

    fn frame(&mut self) {
        todo!()
    }

    fn request_state(&mut self) {
        todo!()
    }
}
