use std::mem::{self, MaybeUninit};
use std::ptr::NonNull;
use std::{error::Error, fmt};

use wlz_macros::{initialization, WlListeners};

use crate::wrapper::wl::{Display, List, Listener};
use crate::wrapper::wlr::{
    Allocator, Backend, BackendEvent, Compositor, Cursor, DataDeviceManager, Output, OutputEvent,
    OutputLayout, OutputState, Renderer, Scene, SceneOutputLayout, SubCompositor, XCursorManager,
    XdgShell, XdgShellEvent,
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

    toplevels: List,
    xdg_shell: XdgShell,
    #[listener("new_xdg_toplevel")]
    new_xdg_toplevel: Listener,

    #[listener("new_xdg_popup")]
    new_xdg_popup: Listener,

    cursor_mgr: XCursorManager,

    cursor: Cursor,

    allocator: Allocator,
    renderer: Renderer,
    backend: Backend,
    display: Display,
}

impl WlzServer {
    #[initialization]
    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        /* The Wayland display is managed by libwayland. It handles accepting
         * clients from the Unix socket, manging Wayland globals, and so on. */
        self.display = Display::try_create()?;

        /* The backend is a wlroots feature which abstracts the underlying input and
         * output hardware. The autocreate option will choose the most suitable
         * backend based on the current environment, such as opening an X11 window
         * if an X11 server is running. */
        self.backend = Backend::autocreate(self.display.get_event_loop())?;

        /* Autocreates a renderer, either Pixman, GLES2 or Vulkan for us. The user
         * can also specify a renderer using the WLR_RENDERER env var.
         * The renderer is responsible for defining the various pixel formats it
         * supports for shared memory, this configures that for clients. */
        self.renderer = Renderer::autocreate(&mut self.backend)?;

        self.renderer.init_wl_display(&mut self.display)?;

        /* Autocreates an allocator for us.
         * The allocator is the bridge between the renderer and the backend. It
         * handles the buffer creation, allowing wlroots to render onto the
         * screen */
        self.allocator = Allocator::autocreate(&mut self.backend, &mut self.renderer)?;

        /* This creates some hands-off wlroots interfaces. The compositor is
         * necessary for clients to allocate surfaces, the subcompositor allows to
         * assign the role of subsurfaces to surfaces and the data device manager
         * handles the clipboard. Each of these wlroots interfaces has room for you
         * to dig your fingers in and play with their behavior if you want. Note that
         * the clients cannot set the selection directly without compositor approval,
         * see the handling of the request_set_selection event below.*/
        Compositor::create(&mut self.display, 5, &mut self.renderer)?;
        SubCompositor::create(&mut self.display)?;
        DataDeviceManager::create(&mut self.display)?;

        /* Creates an output layout, which a wlroots utility for working with an
         * arrangement of screens in a physical layout. */
        self.output_layout = OutputLayout::create(&mut self.display)?;

        /* Configure a listener to be notified when new outputs are available on the
         * backend. */
        self.outputs.init();

        self.init_new_output();
        self.backend
            .get_event_mut(BackendEvent::NewOutput)
            .add(&mut self.new_output);

        /* Create a scene graph. This is a wlroots abstraction that handles all
         * rendering and damage tracking. All the compositor author needs to do
         * is add things that should be rendered to the scene graph at the proper
         * positions and then call wlr_scene_output_commit() to render a frame if
         * necessary.
         */
        self.scene = Scene::create()?;
        self.scene_layout = self.scene.attach_output_layout(&mut self.output_layout)?;

        /* Set up xdg-shell version 3. The xdg-shell is a Wayland protocol which is
         * used for application windows. For more detail on shells, refer to
         * https://drewdevault.com/2018/07/29/Wayland-shells.html.
         */
        self.toplevels.init();
        self.xdg_shell = XdgShell::create(&mut self.display, 3)?;
        self.init_new_xdg_toplevel();
        self.xdg_shell
            .get_event_mut(XdgShellEvent::NewToplevel)
            .add(&mut self.new_xdg_toplevel);
        self.init_new_xdg_popup();
        self.xdg_shell
            .get_event_mut(XdgShellEvent::NewPopup)
            .add(&mut self.new_xdg_popup);

        /*
         * Creates a cursor, which is a wlroots utility for tracking the cursor
         * image shown on screen.
         */
        self.cursor = Cursor::create()?;
        self.cursor.attach_output_layout(&mut self.output_layout);

        /* Creates an xcursor manager, another wlroots utility which loads up
         * Xcursor themes to source cursor images from and makes sure that cursor
         * images are available at all scale factors on the screen (necessary for
         * HiDPI support). */
        self.cursor_mgr = XCursorManager::create(None, 24)?;

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
        let mut pinned_box = Box::pin(MaybeUninit::uninit());
        let output = WlzOutput::initialize(pinned_box.as_mut(), self, wlr_output);
        let output = unsafe { output.get_unchecked_mut() };

        /* Sets up a listener for the frame event. */
        output.init_frame();
        wlr_output
            .get_event_mut(OutputEvent::Frame)
            .add(&mut output.frame);

        /* Sets up a listener for the state request event. */
        output.init_request_state();
        wlr_output
            .get_event_mut(OutputEvent::RequestState)
            .add(&mut output.request_state);

        /* Sets up a listener for the destroy event. */
        output.init_destroy();
        wlr_output
            .get_event_mut(OutputEvent::Destroy)
            .add(&mut output.destroy);

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
        mem::forget(pinned_box);
    }

    fn new_xdg_toplevel(&mut self) {
        unimplemented!()
    }

    fn new_xdg_popup(&mut self) {
        unimplemented!()
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
    #[initialization]
    fn init(&mut self, server: &mut WlzServer, output: &mut Output) {
        self.server = NonNull::new(server as *mut WlzServer).unwrap();
        self.output = NonNull::new(output as *mut Output).unwrap();
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
