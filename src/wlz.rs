use std::mem::{self, MaybeUninit};
use std::pin::Pin;
use std::ptr::NonNull;
use std::{error::Error, fmt};

use pin_project::pin_project;
use wlz_macros::{initialization, WlListeners};

use crate::wrapper::wl::{Display, List, Listener};
use crate::wrapper::wlr::{
    Allocator, Backend, Compositor, Cursor, DataDeviceManager, Output, OutputLayout, OutputState,
    Renderer, Scene, SceneOutputLayout, SubCompositor, XCursorManager, XdgPopup, XdgShell,
    XdgToplevel,
};
use crate::wrapper::WrapperError;
use crate::{destroy_object, error};

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

#[pin_project]
#[derive(WlListeners)]
pub struct WlzServer {
    #[pin]
    outputs: List,
    #[pin]
    #[listener(callback = new_output)]
    new_output: Listener<Output>,
    // field order is important, they are dropped in the order they are declared
    output_layout: OutputLayout,

    scene_layout: SceneOutputLayout,
    scene: Scene,

    #[pin]
    toplevels: List,
    xdg_shell: XdgShell,
    #[pin]
    #[listener(callback = new_xdg_toplevel)]
    new_xdg_toplevel: Listener<XdgToplevel>,

    #[pin]
    #[listener(callback = new_xdg_popup)]
    new_xdg_popup: Listener<XdgPopup>,

    cursor_mgr: XCursorManager,

    cursor: Cursor,

    allocator: Allocator,
    renderer: Renderer,
    backend: Backend,
    display: Display,
}

impl WlzServer {
    #[initialization]
    pub fn init(self: &mut Pin<&mut Self>) -> Result<(), Box<dyn Error>> {
        let this = self.as_mut().project();
        /* The Wayland display is managed by libwayland. It handles accepting
         * clients from the Unix socket, manging Wayland globals, and so on. */
        *this.display = Display::try_create()?;

        /* The backend is a wlroots feature which abstracts the underlying input and
         * output hardware. The autocreate option will choose the most suitable
         * backend based on the current environment, such as opening an X11 window
         * if an X11 server is running. */
        *this.backend = Backend::autocreate(this.display.get_event_loop())?;

        /* Autocreates a renderer, either Pixman, GLES2 or Vulkan for us. The user
         * can also specify a renderer using the WLR_RENDERER env var.
         * The renderer is responsible for defining the various pixel formats it
         * supports for shared memory, this configures that for clients. */
        *this.renderer = Renderer::autocreate(this.backend)?;

        this.renderer.init_wl_display(this.display)?;

        /* Autocreates an allocator for us.
         * The allocator is the bridge between the renderer and the backend. It
         * handles the buffer creation, allowing wlroots to render onto the
         * screen */
        *this.allocator = Allocator::autocreate(this.backend, this.renderer)?;

        /* This creates some hands-off wlroots interfaces. The compositor is
         * necessary for clients to allocate surfaces, the subcompositor allows to
         * assign the role of subsurfaces to surfaces and the data device manager
         * handles the clipboard. Each of these wlroots interfaces has room for you
         * to dig your fingers in and play with their behavior if you want. Note that
         * the clients cannot set the selection directly without compositor approval,
         * see the handling of the request_set_selection event below.*/
        Compositor::create(this.display, 5, this.renderer)?;
        SubCompositor::create(this.display)?;
        DataDeviceManager::create(this.display)?;

        /* Creates an output layout, which a wlroots utility for working with an
         * arrangement of screens in a physical layout. */
        *this.output_layout = OutputLayout::create(this.display)?;

        /* Configure a listener to be notified when new outputs are available on the
         * backend. */
        this.outputs.init();

        this.backend.new_output_event().add(this.new_output);

        /* Create a scene graph. This is a wlroots abstraction that handles all
         * rendering and damage tracking. All the compositor author needs to do
         * is add things that should be rendered to the scene graph at the proper
         * positions and then call wlr_scene_output_commit() to render a frame if
         * necessary.
         */
        *this.scene = Scene::create()?;
        *this.scene_layout = this.scene.attach_output_layout(this.output_layout)?;

        /* Set up xdg-shell version 3. The xdg-shell is a Wayland protocol which is
         * used for application windows. For more detail on shells, refer to
         * https://drewdevault.com/2018/07/29/Wayland-shells.html.
         */
        this.toplevels.init();
        *this.xdg_shell = XdgShell::create(this.display, 3)?;

        this.xdg_shell
            .new_toplevel_event()
            .add(this.new_xdg_toplevel);
        this.xdg_shell.new_popup_event().add(this.new_xdg_popup);

        /*
         * Creates a cursor, which is a wlroots utility for tracking the cursor
         * image shown on screen.
         */
        *this.cursor = Cursor::create()?;
        this.cursor.attach_output_layout(this.output_layout);

        /* Creates an xcursor manager, another wlroots utility which loads up
         * Xcursor themes to source cursor images from and makes sure that cursor
         * images are available at all scale factors on the screen (necessary for
         * HiDPI support). */
        *this.cursor_mgr = XCursorManager::create(None, 24)?;

        Ok(())
    }

    /// This event is raised by the backend when a new output (aka a display or
    /// monitor) becomes available.
    pub fn new_output(mut self: Pin<&mut Self>, mut wlr_output: Pin<&mut Output>) {
        let this = self.as_mut().project();
        /* Configures the output created by the backend to use our allocator
         * and our renderer. Must be done once, before commiting the output */
        wlr_output
            .as_mut()
            .init_renderer(this.allocator, this.renderer);

        /* The output may be disabled, switch it on. */
        let mut state = OutputState::new();
        state.set_enabled(true);

        /* Some backends don't have modes. DRM+KMS does, and we need to set a mode
         * before we can use the output. The mode is a tuple of (width, height,
         * refresh rate), and each monitor supports only a specific set of modes. We
         * just pick the monitor's preferred mode, a more sophisticated compositor
         * would let the user configure it. */
        if let Some(mode) = wlr_output.as_mut().preferred_mode() {
            state.set_mode(mode)
        }

        /* Atomically applies the new output state. */
        wlr_output.as_mut().commit_state(&mut state);
        state.finish();

        /* Allocates and configures our state for this output */
        let mut pinned_box = Box::pin(MaybeUninit::uninit());
        let mut output = WlzOutput::initialize(
            pinned_box.as_mut(),
            unsafe { self.as_mut().get_unchecked_mut() },
            unsafe { wlr_output.as_mut().get_unchecked_mut() },
        );
        // reborrow this again
        let this = self.as_mut().project();
        //let output = unsafe { output.get_unchecked_mut() };

        /* Sets up a listener for the frame event. */
        wlr_output
            .as_mut()
            .frame_event()
            .add(output.as_mut().project().frame);

        /* Sets up a listener for the state request event. */
        wlr_output
            .as_mut()
            .request_state_event()
            .add(output.as_mut().project().request_state);

        /* Sets up a listener for the destroy event. */
        wlr_output
            .as_mut()
            .destroy_event()
            .add(output.as_mut().project().destroy);

        this.outputs.insert(output.project().link);

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
            let mut l_output = this.output_layout.add_auto(wlr_output.as_mut())?;

            let mut scene_output = this.scene.output_create(wlr_output)?;
            this.scene_layout
                .add_output(&mut l_output, &mut scene_output);
            Ok::<(), WrapperError>(())
        })() {
            error!("Failure during adding of scene output: {}", e);
        }

        // forget the memory, it is deallocated when destroy signal is received
        mem::forget(pinned_box);
    }

    fn new_xdg_toplevel(self: Pin<&mut Self>, _xdg_toplevel: Pin<&mut XdgToplevel>) {
        unimplemented!()
    }

    fn new_xdg_popup(self: Pin<&mut Self>, _xdg_popup: Pin<&mut XdgPopup>) {
        unimplemented!()
    }

    /*pub fn display(&self) -> &Display {
        &self.display
    }*/
}

#[derive(WlListeners)]
#[pin_project]
struct WlzOutput {
    #[pin]
    link: List,
    server: NonNull<WlzServer>,
    output: NonNull<Output>,
    #[pin]
    #[listener(callback = frame)]
    frame: Listener,
    #[pin]
    #[listener(callback = request_state)]
    request_state: Listener,
    #[pin]
    #[listener(callback = destroy)]
    destroy: Listener,
}

impl WlzOutput {
    #[initialization]
    fn init(self: &mut Pin<&mut Self>, server: &mut WlzServer, output: &mut Output) {
        *self.as_mut().project().server = NonNull::new(server as *mut WlzServer).unwrap();
        *self.as_mut().project().output = NonNull::new(output as *mut Output).unwrap();
    }

    fn destroy(self: Pin<&mut Self>) {
        unsafe { destroy_object(self) };
    }

    fn frame(self: Pin<&mut Self>) {
        todo!()
    }

    fn request_state(self: Pin<&mut Self>) {
        todo!()
    }
}
