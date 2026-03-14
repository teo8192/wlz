use std::mem::{self, MaybeUninit};
use std::pin::Pin;
use std::{error::Error, fmt};

use pin_project::pin_project;
use wlz_macros::{initialization, WlListeners};

use crate::wrapper::wl::{Display, List, Listener};
use crate::wrapper::wlr::{
    Allocator, Backend, Compositor, Cursor, DataDeviceManager, DataField, Output, OutputLayout,
    OutputState, Renderer, Scene, SceneOutputLayout, SceneTree, SubCompositor, XCursorManager,
    XdgPopup, XdgShell, XdgToplevel,
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

enum WlzCursorMode {
    Passthough,
    Move,
    Resize,
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

    cursor_mode: WlzCursorMode,

    #[pin]
    #[listener(callback = cursor_motion)]
    cursor_motion: Listener,
    #[pin]
    #[listener(callback = cursor_motion_absolute)]
    cursor_motion_absolute: Listener,
    #[pin]
    #[listener(callback = cursor_button)]
    cursor_button: Listener,
    #[pin]
    #[listener(callback = cursor_axis)]
    cursor_axis: Listener,
    #[pin]
    #[listener(callback = cursor_frame)]
    cursor_frame: Listener,

    cursor_mgr: XCursorManager,
    cursor: Cursor,
    allocator: Allocator,
    renderer: Renderer,
    backend: Backend,
    display: Display,
}

impl WlzServer {
    #[initialization]
    pub fn init(mut self: Pin<&mut Self>) -> Result<(), Box<dyn Error>> {
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

        /*
         * wlr_cursor *only* displays an image on screen. It does not move around
         * when the pointer moves. However, we can attach input devices to it, and
         * it will generate aggregate events for all of them. In these events, we
         * can choose how we want to process them, forwarding them to clients and
         * moving the cursor around. More detail on this process is described in
         * https://drewdevault.com/2018/07/17/Input-handling-in-wlroots.html.
         *
         * And more comments are sprinkled throughout the notify functions above.
         */
        *this.cursor_mode = WlzCursorMode::Passthough;
        this.cursor.motion_event().add(this.cursor_motion);
        this.cursor.motion_absolute_event().add(this.cursor_motion_absolute);
        this.cursor.button_event().add(this.cursor_button);
        this.cursor.axis_event().add(this.cursor_axis);
        this.cursor.frame_event().add(this.cursor_frame);

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
        let mut output =
            WlzOutput::initialize(pinned_box.as_mut(), self.as_mut(), wlr_output.as_mut());

        // reborrow these again
        let output = output.as_mut().project();
        let this = output.server.as_mut().project();
        let wlr_output = output.output;

        // insert the output into our list of outputs
        this.outputs.insert(output.link);

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

            let mut scene_output = this.scene.output_create(wlr_output.as_mut())?;
            this.scene_layout
                .add_output(&mut l_output, &mut scene_output);
            Ok::<(), WrapperError>(())
        })() {
            error!("Failure during adding of scene output: {}", e);
        }

        // forget the memory, it is deallocated when destroy signal is received
        mem::forget(pinned_box);
    }

    /// This event is raised when a client creates a new toplevel (application window).
    fn new_xdg_toplevel(self: Pin<&mut Self>, xdg_toplevel: Pin<&mut XdgToplevel>) {
        /* Allocate a WlzToplevel for this surface */

        let mut pinned_box = Box::pin(MaybeUninit::uninit());
        WlzToplevel::initialize(pinned_box.as_mut(), self, xdg_toplevel);

        mem::forget(pinned_box);
    }

    /// This event is raised when a client creates a new popup.
    fn new_xdg_popup(self: Pin<&mut Self>, mut xdg_popup: Pin<&mut XdgPopup>) {
        let mut pinned_box = Box::pin(MaybeUninit::uninit());
        let mut popup = WlzPopup::initialize(pinned_box.as_mut(), xdg_popup.as_mut());

        let popup = popup.as_mut().project();

        let mut xdg_popup = popup.xdg_popup.as_mut();

        /* We must add xdg popups to the scene graph so they get rendered. The
         * wlroots scene graph provides a helper for this, but to use it we must
         * provide the proper parent scene node of the xdg popup. To enable this,
         * we always set the user data field of xdg_surfaces to the corresponding
         * scene node. */
        let mut parent = xdg_popup.as_mut().parent().expect("XdgPopup had no parent");
        let parent_tree = parent.data::<SceneTree>().unwrap();
        let new_xdg_surface = parent_tree
            .xdg_surface_create(xdg_popup.as_mut().base().unwrap())
            .unwrap();
        xdg_popup
            .as_mut()
            .base()
            .unwrap()
            .set_data(new_xdg_surface.as_ref());

        xdg_popup
            .as_mut()
            .base()
            .unwrap()
            .surface()
            .commit_event()
            .add(popup.commit);

        // forget the memory, it is deallocated when destroy signal is received
        mem::forget(pinned_box);
    }

    fn cursor_motion(self: Pin<&mut Self>) {
        todo!()
    }

    fn cursor_motion_absolute(self: Pin<&mut Self>) {
        todo!()
    }

    fn cursor_button(self: Pin<&mut Self>) {
        todo!()
    }

    fn cursor_axis(self: Pin<&mut Self>) {
        todo!()
    }

    fn cursor_frame(self: Pin<&mut Self>) {
        todo!()
    }
}

#[derive(WlListeners)]
#[pin_project]
struct WlzOutput<'a, 'b> {
    #[pin]
    link: List,
    server: Pin<&'a mut WlzServer>,
    output: Pin<&'b mut Output>,
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

impl<'a, 'b> WlzOutput<'a, 'b> {
    #[initialization]
    fn init(mut self: Pin<&mut Self>, server: Pin<&'a mut WlzServer>, output: Pin<&'b mut Output>) {
        let this = self.as_mut().project();
        *this.server = server;
        *this.output = output;

        /* Sets up a listener for the frame event. */
        this.output.as_mut().frame_event().add(this.frame);

        /* Sets up a listener for the state request event. */
        this.output
            .as_mut()
            .request_state_event()
            .add(this.request_state);

        /* Sets up a listener for the destroy event. */
        this.output.as_mut().destroy_event().add(this.destroy);
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

#[derive(WlListeners)]
#[pin_project]
struct WlzPopup<'a> {
    xdg_popup: Pin<&'a mut XdgPopup>,

    #[pin]
    #[listener(callback = commit)]
    commit: Listener,

    #[pin]
    #[listener(callback = destroy)]
    destroy: Listener,
}

impl<'a> WlzPopup<'a> {
    #[initialization]
    fn init(mut self: Pin<&mut Self>, popup: Pin<&'a mut XdgPopup>) {
        let this = self.as_mut().project();
        *this.xdg_popup = popup;

        this.xdg_popup.as_mut().destroy_event().add(this.destroy);
    }

    fn commit(self: Pin<&mut Self>) {
        todo!()
    }

    fn destroy(self: Pin<&mut Self>) {
        unsafe { destroy_object(self) };
    }
}

#[derive(WlListeners)]
#[pin_project]
struct WlzToplevel<'a, 'b, 'c> {
    #[pin]
    link: List,
    server: Pin<&'a mut WlzServer>,
    xdg_toplevel: Pin<&'b mut XdgToplevel>,
    scene_tree: Pin<&'c mut SceneTree>,
    #[pin]
    #[listener(callback = map)]
    map: Listener,
    #[pin]
    #[listener(callback = unmap)]
    unmap: Listener,
    #[pin]
    #[listener(callback = commit)]
    commit: Listener,
    #[pin]
    #[listener(callback = destroy)]
    destroy: Listener,
    #[pin]
    #[listener(callback = request_move)]
    request_move: Listener,
    #[pin]
    #[listener(callback = request_resize)]
    request_resize: Listener,
    #[pin]
    #[listener(callback = request_maximize)]
    request_maximize: Listener,
    #[pin]
    #[listener(callback = request_fullscreen)]
    request_fullscreen: Listener,
}

impl<'a, 'b, 'c> WlzToplevel<'a, 'b, 'c>
where
    'b: 'c,
{
    #[initialization]
    fn init(
        mut self: Pin<&mut Self>,
        server: Pin<&'a mut WlzServer>,
        xdg_toplevel: Pin<&'b mut XdgToplevel>,
    ) {
        let self_ptr = self.as_ref().get_ref() as *const Self;
        let this = self.as_mut().project();
        *this.server = server;
        *this.xdg_toplevel = xdg_toplevel;

        *this.scene_tree = this
            .server
            .as_mut()
            .project()
            .scene
            .tree()
            .xdg_surface_create(this.xdg_toplevel.as_mut().base())
            .unwrap();

        this.scene_tree
            .as_mut()
            .project()
            .node
            .pin_set_data_ptr(self_ptr);

        this.xdg_toplevel
            .as_mut()
            .base()
            .set_data(this.scene_tree.as_ref());

        /* Listen to the various events it can emit */
        let mut surface = this.xdg_toplevel.as_mut().base().surface();
        surface.map_event().add(this.map);
        surface.unmap_event().add(this.unmap);
        surface.unmap_event().add(this.commit);

        this.xdg_toplevel.as_mut().destroy_event().add(this.destroy);
        this.xdg_toplevel
            .as_mut()
            .request_move_event()
            .add(this.request_move);
        this.xdg_toplevel
            .as_mut()
            .request_resize_event()
            .add(this.request_resize);
        this.xdg_toplevel
            .as_mut()
            .request_maximize_event()
            .add(this.request_maximize);
        this.xdg_toplevel
            .as_mut()
            .request_fullscreen_event()
            .add(this.request_fullscreen);
    }

    fn focus(self: Pin<&mut Self>) {
        todo!()
    }

    fn destroy(self: Pin<&mut Self>) {
        unsafe { destroy_object(self) };
    }

    /// Called when the surface is mapped, or ready to display on-screen.
    fn map(mut self: Pin<&mut Self>) {
        let this = self.as_mut().project();

        this.server.as_mut().project().toplevels.insert(this.link);

        self.focus();
    }

    /// Called when the surface is unmapped, and should no longer be shown.
    fn unmap(self: Pin<&mut Self>) {
        /* Reset the cursor mode if the grabbed toplevel was unmapped. */
        todo!();
        /*if (toplevel == toplevel->server->grabbed_toplevel) {
            reset_cursor_mode(toplevel->server);
        }*/

        #[allow(unreachable_code)]
        self.project().link.remove()
    }

    fn commit(self: Pin<&mut Self>) {
        todo!()
    }

    fn request_move(self: Pin<&mut Self>) {
        todo!()
    }

    fn request_resize(self: Pin<&mut Self>) {
        todo!()
    }

    fn request_maximize(self: Pin<&mut Self>) {
        todo!()
    }

    fn request_fullscreen(self: Pin<&mut Self>) {
        todo!()
    }
}
