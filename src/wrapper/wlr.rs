use std::error::Error;
use std::ffi::CString;
use std::marker::PhantomPinned;
use std::mem::zeroed;
use std::pin::Pin;
use std::ptr::{null, null_mut, NonNull};
use std::str::FromStr;

use wlz_macros::{c_drop, FromPtr, PtrWrapper};

use super::wl::EventLoop;
use super::WrapperError;
use crate::ffi;
use crate::wrapper::wl::{Display, Signal};

/// A backend provides a set of input and output devices.
///
/// Buffer capabilities and features can change over the lifetime of a backend,
/// for instance when a child backend is added to a multi-backend.
#[derive(PtrWrapper)]
#[c_drop(ffi::wlr_backend_destroy)]
pub struct Backend(NonNull<ffi::wlr_backend>);

impl Backend {
    /// Automatically initializes the most suitable backend given the environment.
    /// Will always return a multi-backend. The backend is created but not started.
    /// Returns NULL on failure.
    ///
    /// If session_ptr is not NULL, it's populated with the session which has been
    /// created with the backend, if any.
    ///
    /// The multi-backend will be destroyed if one of the primary underlying
    /// backends is destroyed (e.g. if the primary DRM device is unplugged).
    pub fn autocreate(event_loop: EventLoop) -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_backend_autocreate(event_loop.as_ptr(), null_mut()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateBackend)
    }

    pub fn new_output_event(&mut self) -> Pin<&mut Signal<Output>> {
        unsafe {
            Signal::get_event_mut(Pin::new_unchecked(self.as_mut()), |v| {
                &mut v.events.new_output
            })
        }
    }
}

#[doc = "A renderer for basic 2D operations."]
#[derive(PtrWrapper)]
#[c_drop(ffi::wlr_renderer_destroy)]
pub struct Renderer(NonNull<ffi::wlr_renderer>);

impl Renderer {
    #[doc = "Automatically create a new renderer.\n\n Selects an appropriate renderer type to use depending on the backend,\n platform, environment, etc."]
    pub fn autocreate(backend: &mut Backend) -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_renderer_autocreate(backend.as_ptr()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateRenderer)
    }

    #[doc = "Initializes wl_shm, linux-dmabuf and other buffer factory protocols.\n\n Returns false on failure."]
    pub fn init_wl_display(&mut self, wl_display: &mut Display) -> Result<(), WrapperError> {
        if unsafe { ffi::wlr_renderer_init_wl_display(self.as_ptr(), wl_display.as_ptr()) } {
            Ok(())
        } else {
            Err(WrapperError::FailedToInitializeDisplay)
        }
    }
}

#[doc = "An allocator is responsible for allocating memory for pixel buffers.\n\n Each allocator may return buffers with different capabilities (shared\n memory, DMA-BUF, memory mapping, etc), placement (main memory, VRAM on a\n GPU, etc) and properties (possible usage, access performance, etc). See\n struct wlr_buffer.\n\n An allocator can be passed to a struct wlr_swapchain for multiple buffering."]
#[derive(PtrWrapper)]
#[c_drop(ffi::wlr_allocator_destroy)]
pub struct Allocator(NonNull<ffi::wlr_allocator>);

impl Allocator {
    #[doc = "Creates the adequate struct wlr_allocator given a backend and a renderer."]
    pub fn autocreate(
        wlr_backend: &mut Backend,
        wlr_renderer: &mut Renderer,
    ) -> Result<Self, WrapperError> {
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
    #[doc = "Create the wl_compositor global, which can be used by clients to create\n surfaces and regions.\n\n If a renderer is supplied, the compositor will create struct wlr_texture\n objects from client buffers on surface commit."]
    pub fn create(
        wl_display: &mut Display,
        version: u32,
        wlr_renderer: &mut Renderer,
    ) -> Result<Self, WrapperError> {
        NonNull::new(unsafe {
            ffi::wlr_compositor_create(wl_display.as_ptr(), version, wlr_renderer.as_ptr())
        })
        .map(Self)
        .ok_or(WrapperError::FailedToCreateCompositor)
    }
}

#[derive(PtrWrapper)]
pub struct SubCompositor(NonNull<ffi::wlr_subcompositor>);

impl SubCompositor {
    pub fn create(wl_display: &mut Display) -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_subcompositor_create(wl_display.as_ptr()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateSubCompositor)
    }
}

#[derive(PtrWrapper)]
pub struct DataDeviceManager(NonNull<ffi::wlr_data_device_manager>);

impl DataDeviceManager {
    #[doc = "Create a wl_data_device_manager global for this display."]
    pub fn create(wl_display: &mut Display) -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_data_device_manager_create(wl_display.as_ptr()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateDataDeviceManager)
    }
}

#[doc = "Helper to arrange outputs in a 2D coordinate space. The output effective\n resolution is used, see wlr_output_effective_resolution().\n\n Outputs added to the output layout are automatically exposed to clients (see\n wlr_output_create_global()). They are no longer exposed when removed from the\n layout."]
#[derive(PtrWrapper)]
pub struct OutputLayout(NonNull<ffi::wlr_output_layout>);

impl OutputLayout {
    pub fn create(wl_display: &mut Display) -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_output_layout_create(wl_display.as_ptr()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateOutputLayout)
    }

    /// Add the output to the layout as automatically configured. This will place
    /// the output in a sensible location in the layout. The coordinates of
    /// the output in the layout will be adjusted dynamically when the layout
    /// changes. If the output is already a part of the layout, it will become
    /// automatically configured.
    ///
    /// Returns the output's output layout, or NULL on error.
    pub fn add_auto(
        &mut self,
        output: Pin<&mut Output>,
    ) -> Result<OutputLayoutOutput, WrapperError> {
        NonNull::new(unsafe {
            ffi::wlr_output_layout_add_auto(self.as_ptr(), output.get_unchecked_mut().as_ptr())
        })
        .map(OutputLayoutOutput)
        .ok_or(WrapperError::FailedOutputLayoutAddAuto)
    }
}

#[derive(PtrWrapper)]
pub struct OutputLayoutOutput(NonNull<ffi::wlr_output_layout_output>);

/// A compositor output region. This typically corresponds to a monitor that
/// displays part of the compositor space.
///
/// The `frame` event will be emitted when it is a good time for the compositor
/// to submit a new frame.
///
/// To render a new frame compositors should call wlr_output_begin_render_pass(),
/// perform rendering on that render pass, and finally call
/// wlr_output_commit_state()."]
#[derive(FromPtr)]
pub struct Output(ffi::wlr_output, PhantomPinned);

impl Output {
    /// Initialize the output's rendering subsystem with the provided allocator and
    /// renderer. After initialization, this function may invoked again to reinitialize
    /// the allocator and renderer to different values.
    ///
    /// Call this function prior to any call to wlr_output_begin_render_pass(),
    /// wlr_output_commit_state() or wlr_output_cursor_create().
    ///
    /// The buffer capabilities of the provided must match the capabilities of the
    /// output's backend. Returns false otherwise.
    pub fn init_renderer(self: Pin<&mut Self>, allocator: &mut Allocator, renderer: &mut Renderer) {
        // TODO: handle error
        unsafe {
            ffi::wlr_output_init_render(
                self.get_unchecked_mut().as_ptr(),
                allocator.as_ptr(),
                renderer.as_ptr(),
            )
        };
    }

    /// Returns the preferred mode for this output. If the output doesn't support
    /// modes, returns NULL.
    pub fn preferred_mode(self: Pin<&mut Self>) -> Option<Pin<&mut OutputMode>> {
        let mode = unsafe { ffi::wlr_output_preferred_mode(self.get_unchecked_mut().as_ptr()) };

        NonNull::new(mode).map(|m| unsafe { Pin::new_unchecked(OutputMode::from_ptr(m)) })
    }

    /// Attempts to apply the state to this output. This function may fail for any
    /// reason and return false. If failed, none of the state would have been applied,
    /// this function is atomic. If the commit succeeded, true is returned.
    ///
    /// Note: wlr_output_state_finish() would typically be called after the state
    /// has been committed.
    pub fn commit_state(self: Pin<&mut Self>, state: &mut OutputState) {
        // TODO: handle error
        unsafe { ffi::wlr_output_commit_state(self.get_unchecked_mut().as_ptr(), state.as_ptr()) };
    }

    pub fn frame_event(self: Pin<&mut Self>) -> Pin<&mut Signal> {
        // SAFETY: This is safe since the interior does not move out of the reference,
        // and the returned value does not move since it is a field of the pinned value
        unsafe { Signal::get_event_mut(self, |v| &mut v.0.events.frame) }
    }

    pub fn request_state_event(self: Pin<&mut Self>) -> Pin<&mut Signal> {
        // SAFETY: This is safe since the interior does not move out of the reference,
        // and the returned value does not move since it is a field of the pinned value
        unsafe { Signal::get_event_mut(self, |v| &mut v.0.events.request_state) }
    }

    pub fn destroy_event(self: Pin<&mut Self>) -> Pin<&mut Signal> {
        // SAFETY: This is safe since the interior does not move out of the reference,
        // and the returned value does not move since it is a field of the pinned value
        unsafe { Signal::get_event_mut(self, |v| &mut v.0.events.destroy) }
    }
}

#[doc = "Holds the double-buffered output state."]
#[derive(FromPtr)]
pub struct OutputState(ffi::wlr_output_state);

impl OutputState {
    #[doc = "Initialize an output state."]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut output_state = Self(unsafe { zeroed() });
        unsafe { ffi::wlr_output_state_init(output_state.as_ptr()) };

        output_state
    }

    /// Enables or disables an output. A disabled output is turned off and doesn't
    /// emit `frame` events.
    ///
    /// This state will be applied once wlr_output_commit_state() is called.
    pub fn set_enabled(&mut self, enabled: bool) {
        unsafe { ffi::wlr_output_state_set_enabled(self.as_ptr(), enabled) };
    }

    /// Sets the output mode of an output. An output mode will specify the resolution
    /// and refresh rate, among other things.
    ///
    /// This state will be applied once wlr_output_commit_state() is called.
    pub fn set_mode(&mut self, output_mode: Pin<&mut OutputMode>) {
        unsafe {
            ffi::wlr_output_state_set_mode(self.as_ptr(), output_mode.get_unchecked_mut().as_ptr())
        };
    }

    #[doc = "Releases all resources associated with an output state."]
    pub fn finish(&mut self) {
        unsafe { ffi::wlr_output_state_finish(self.as_ptr()) };
    }
}

#[derive(FromPtr)]
pub struct OutputMode(ffi::wlr_output_mode, PhantomPinned);

#[derive(PtrWrapper)]
pub struct SceneOutputLayout(NonNull<ffi::wlr_scene_output_layout>);

impl SceneOutputLayout {
    #[doc = "Add an output to the scene output layout.\n\n When the layout output is repositioned, the scene output will be repositioned\n accordingly."]
    pub fn add_output(
        &mut self,
        layout_output: &mut OutputLayoutOutput,
        scene_output: &mut SceneOutput,
    ) {
        unsafe {
            ffi::wlr_scene_output_layout_add_output(
                self.as_ptr(),
                layout_output.as_ptr(),
                scene_output.as_ptr(),
            )
        };
    }
}

#[doc = "A viewport for an output in the scene-graph"]
#[derive(PtrWrapper)]
pub struct SceneOutput(NonNull<ffi::wlr_scene_output>);

#[doc = "The root scene-graph node."]
#[derive(PtrWrapper)]
pub struct Scene(NonNull<ffi::wlr_scene>);

impl Scene {
    #[doc = "Create a new scene-graph.\n\n The graph is also a struct wlr_scene_node. Associated resources can be\n destroyed through wlr_scene_node_destroy()."]
    pub fn create() -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_scene_create() })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateScene)
    }

    /// Add a viewport for the specified output to the scene-graph.
    ///
    /// An output can only be added once to the scene-graph.
    pub fn output_create(&mut self, output: Pin<&mut Output>) -> Result<SceneOutput, WrapperError> {
        NonNull::new(unsafe {
            ffi::wlr_scene_output_create(self.as_ptr(), output.get_unchecked_mut().as_ptr())
        })
        .map(SceneOutput)
        .ok_or(WrapperError::FailedToCreateSceneOutput)
    }

    #[doc = "Attach an output layout to a scene.\n\n The resulting scene output layout allows to synchronize the positions of scene\n outputs with the positions of corresponding layout outputs.\n\n It is automatically destroyed when the scene or the output layout is destroyed."]
    pub fn attach_output_layout(
        &mut self,
        output_layout: &mut OutputLayout,
    ) -> Result<SceneOutputLayout, WrapperError> {
        NonNull::new(unsafe {
            ffi::wlr_scene_attach_output_layout(self.as_ptr(), output_layout.as_ptr())
        })
        .map(SceneOutputLayout)
        .ok_or_else(|| {
            WrapperError::GeneralError("failed to attach output layout to scene".to_string())
        })
    }
}

pub enum XdgShellEvent {
    NewSurface,
    NewToplevel,
    NewPopup,
    Destroy,
}

// TODO: make this pin and make sure the inner type is not unpin
#[derive(PtrWrapper)]
pub struct XdgShell(NonNull<ffi::wlr_xdg_shell>);

impl XdgShell {
    /// Create the xdg_wm_base global with the specified version.
    pub fn create(display: &mut Display, version: u32) -> Result<XdgShell, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_xdg_shell_create(display.as_ptr(), version) })
            .map(Self)
            .ok_or_else(|| WrapperError::GeneralError("failed to create xdg shell".to_string()))
    }

    pub fn new_toplevel_event(&mut self) -> Pin<&mut Signal<XdgToplevel>> {
        unsafe {
            Signal::get_event_mut(Pin::new_unchecked(self.as_mut()), |v| {
                &mut v.events.new_toplevel
            })
        }
    }

    pub fn new_popup_event(&mut self) -> Pin<&mut Signal<XdgPopup>> {
        unsafe {
            Signal::get_event_mut(Pin::new_unchecked(self.as_mut()), |v| {
                &mut v.events.new_popup
            })
        }
    }
}

#[c_drop(ffi::wlr_cursor_destroy)]
#[derive(PtrWrapper)]
pub struct Cursor(NonNull<ffi::wlr_cursor>);

impl Cursor {
    pub fn create() -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_cursor_create() })
            .map(Self)
            .ok_or_else(|| WrapperError::GeneralError("Failed to create cursor".to_string()))
    }

    #[doc = "Uses the given layout to establish the boundaries and movement semantics of\n this cursor. Cursors without an output layout allow infinite movement in any\n direction and do not support absolute input events."]
    pub fn attach_output_layout(&mut self, output_layout: &mut OutputLayout) {
        unsafe { ffi::wlr_cursor_attach_output_layout(self.as_ptr(), output_layout.as_ptr()) };
    }
}

/// struct wlr_xcursor_manager dynamically loads xcursor themes at sizes necessary
/// for use on outputs at arbitrary scale factors. You should call
/// wlr_xcursor_manager_load() for each output you will show your cursor on, with
/// the scale factor parameter set to that output's scale factor.
#[c_drop(ffi::wlr_xcursor_manager_destroy)]
#[derive(PtrWrapper)]
pub struct XCursorManager(NonNull<ffi::wlr_xcursor_manager>);

impl XCursorManager {
    /// Creates a new XCursor manager with the given xcursor theme name and base size
    /// (for use when scale=1).
    pub fn create(name: Option<&str>, size: u32) -> Result<Self, Box<dyn Error>> {
        let nn = if let Some(name) = name {
            CString::from_str(name)
        } else {
            CString::from_str("")
        }?;
        let ptr = if name.is_some() { nn.as_ptr() } else { null() };
        Ok(
            NonNull::new(unsafe { ffi::wlr_xcursor_manager_create(ptr, size) })
                .map(Self)
                .ok_or_else(|| {
                    WrapperError::GeneralError("Failed to create xcursor manager".to_string())
                })?,
        )
    }
}

#[derive(FromPtr)]
pub struct XdgToplevel(ffi::wlr_xdg_toplevel);

#[derive(FromPtr)]
pub struct XdgPopup(ffi::wlr_xdg_popup, PhantomPinned);

impl XdgPopup {
    pub fn destroy_event(&mut self) -> Pin<&mut Signal> {
        // SAFETY: This is safe since the interior does not move out of the reference,
        // and the returned value does not move since it is a field of the pinned value
        unsafe { Signal::get_event_mut(Pin::new_unchecked(self), |v| &mut v.0.events.destroy) }
    }

    pub fn parent(self: Pin<&mut XdgPopup>) -> Option<Surface> {
        self.0.parent.try_into().ok()
    }

    pub fn base(self: Pin<&mut XdgPopup>) -> Option<XdgSurface> {
        self.0.base.try_into().ok()
    }
}

#[derive(PtrWrapper)]
pub struct Surface(NonNull<ffi::wlr_surface>);

impl Surface {
    pub fn data<T>(&self) -> Option<NonNull<T>> {
        NonNull::new(self.as_ref().data as *mut T)
    }

    pub fn commit_event(&mut self) -> Pin<&mut Signal> {
        unsafe {
            Signal::get_event_mut(Pin::new_unchecked(self), |v| {
                &mut v.0.as_mut().events.commit
            })
        }
    }
}

#[derive(PtrWrapper)]
pub struct SceneTree(NonNull<ffi::wlr_scene_tree>);

impl SceneTree {
    /// Add a node displaying an xdg_surface and all of its sub-surfaces to the
    /// scene-graph.
    ///
    /// The origin of the returned scene-graph node will match the top-left corner
    /// of the xdg_surface window geometry.
    pub fn xdg_surface_create(&mut self, xdg_surface: XdgSurface) -> Option<Self> {
        unsafe { ffi::wlr_scene_xdg_surface_create(self.as_ptr(), xdg_surface.as_ptr()) }
            .try_into()
            .ok()
    }
}

#[derive(PtrWrapper)]
/// An xdg-surface is a user interface element requiring management by the
/// compositor. An xdg-surface alone isn't useful, a role should be assigned to
/// it in order to map it.
pub struct XdgSurface(NonNull<ffi::wlr_xdg_surface>);

impl XdgSurface {
    pub fn set_data<T>(&mut self, data: &mut T) {
        self.as_mut().data = data as *mut T as *mut std::os::raw::c_void;
    }

    pub fn surface(&mut self) -> Surface {
        unsafe { self.0.as_mut() }.surface.try_into().unwrap()
    }
}
