use std::mem::zeroed;
use std::ptr::{null_mut, NonNull};

use wlz_macros::{c_drop, FromPtr, PtrWrapper};

use super::wl::EventLoop;
use super::WrapperError;
use crate::ffi;
use crate::wrapper::wl::{Display, Signal};

#[doc = "A backend provides a set of input and output devices.\n\n Buffer capabilities and features can change over the lifetime of a backend,\n for instance when a child backend is added to a multi-backend."]
#[derive(PtrWrapper)]
#[c_drop(ffi::wlr_backend_destroy)]
pub struct Backend(NonNull<ffi::wlr_backend>);

pub enum BackendEvent {
    Destroy,
    NewInput,
    NewOutput,
}

impl Backend {
    #[doc = "Automatically initializes the most suitable backend given the environment.\n Will always return a multi-backend. The backend is created but not started.\n Returns NULL on failure.\n\n If session_ptr is not NULL, it's populated with the session which has been\n created with the backend, if any.\n\n The multi-backend will be destroyed if one of the primary underlying\n backends is destroyed (e.g. if the primary DRM device is unplugged)."]
    pub fn autocreate(event_loop: EventLoop) -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_backend_autocreate(event_loop.as_ptr(), null_mut()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateBackend)
    }

    pub fn get_event(&self, event: BackendEvent) -> &Signal {
        use BackendEvent::*;
        let backend = unsafe { self.0.as_ref() };
        let event_ptr = match event {
            Destroy => &backend.events.destroy,
            NewInput => &backend.events.new_input,
            NewOutput => &backend.events.new_output,
        } as *const ffi::wl_signal;
        let signal_ptr = event_ptr as *const Signal;
        unsafe { &(*signal_ptr) as &Signal }
    }

    pub fn get_event_mut(&mut self, event: BackendEvent) -> &mut Signal {
        use BackendEvent::*;
        let backend = unsafe { self.0.as_mut() };
        let event_ptr = match event {
            Destroy => &mut backend.events.destroy,
            NewInput => &mut backend.events.new_input,
            NewOutput => &mut backend.events.new_output,
        } as *mut ffi::wl_signal;
        let signal_ptr = event_ptr as *mut Signal;
        unsafe { &mut (*signal_ptr) as &mut Signal }
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

    #[doc = "Add the output to the layout as automatically configured. This will place\n the output in a sensible location in the layout. The coordinates of\n the output in the layout will be adjusted dynamically when the layout\n changes. If the output is already a part of the layout, it will become\n automatically configured.\n\n Returns the output's output layout, or NULL on error."]
    pub fn add_auto(&mut self, output: &mut Output) -> Result<OutputLayoutOutput, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_output_layout_add_auto(self.as_ptr(), output.as_ptr()) })
            .map(OutputLayoutOutput)
            .ok_or(WrapperError::FailedOutputLayoutAddAuto)
    }
}

#[derive(PtrWrapper)]
pub struct OutputLayoutOutput(NonNull<ffi::wlr_output_layout_output>);

pub enum OutputEvent {
    Frame,
    RequestState,
    Destroy,
}

#[doc = "A compositor output region. This typically corresponds to a monitor that\n displays part of the compositor space.\n\n The `frame` event will be emitted when it is a good time for the compositor\n to submit a new frame.\n\n To render a new frame compositors should call wlr_output_begin_render_pass(),\n perform rendering on that render pass, and finally call\n wlr_output_commit_state()."]
#[derive(FromPtr)]
pub struct Output(ffi::wlr_output);

impl Output {
    #[doc = "Initialize the output's rendering subsystem with the provided allocator and\n renderer. After initialization, this function may invoked again to reinitialize\n the allocator and renderer to different values.\n\n Call this function prior to any call to wlr_output_begin_render_pass(),\n wlr_output_commit_state() or wlr_output_cursor_create().\n\n The buffer capabilities of the provided must match the capabilities of the\n output's backend. Returns false otherwise."]
    pub fn init_renderer(&mut self, allocator: &mut Allocator, renderer: &mut Renderer) {
        // TODO: handle error
        unsafe {
            ffi::wlr_output_init_render(self.as_ptr(), allocator.as_ptr(), renderer.as_ptr())
        };
    }

    #[doc = "Returns the preferred mode for this output. If the output doesn't support\n modes, returns NULL."]
    pub fn preferred_mode(&mut self) -> Option<&mut OutputMode> {
        let mode = unsafe { ffi::wlr_output_preferred_mode(self.as_ptr()) };

        NonNull::new(mode).map(|m| unsafe { OutputMode::from_ptr(m) })
    }

    #[doc = "Attempts to apply the state to this output. This function may fail for any\n reason and return false. If failed, none of the state would have been applied,\n this function is atomic. If the commit succeeded, true is returned.\n\n Note: wlr_output_state_finish() would typically be called after the state\n has been committed."]
    pub fn commit_state(&mut self, state: &mut OutputState) {
        // TODO: handle error
        unsafe { ffi::wlr_output_commit_state(self.as_ptr(), state.as_ptr()) };
    }

    pub fn get_event(&self, ty: OutputEvent) -> &Signal {
        use OutputEvent::*;
        let event_ptr = match ty {
            Frame => &self.0.events.frame,
            RequestState => &self.0.events.request_state,
            Destroy => &self.0.events.destroy,
        } as *const ffi::wl_signal;
        let signal_ptr = event_ptr as *const Signal;
        unsafe { &(*signal_ptr) as &Signal }
    }

    pub fn get_event_mut(&mut self, ty: OutputEvent) -> &mut Signal {
        use OutputEvent::*;
        let event_ptr = match ty {
            Frame => &mut self.0.events.frame,
            RequestState => &mut self.0.events.request_state,
            Destroy => &mut self.0.events.destroy,
        } as *mut ffi::wl_signal;
        let signal_ptr = event_ptr as *mut Signal;
        unsafe { &mut (*signal_ptr) as &mut Signal }
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

    #[doc = "Enables or disables an output. A disabled output is turned off and doesn't\n emit `frame` events.\n\n This state will be applied once wlr_output_commit_state() is called."]
    pub fn set_enabled(&mut self, enabled: bool) {
        unsafe { ffi::wlr_output_state_set_enabled(self.as_ptr(), enabled) };
    }

    #[doc = "Sets the output mode of an output. An output mode will specify the resolution\n and refresh rate, among other things.\n\n This state will be applied once wlr_output_commit_state() is called."]
    pub fn set_mode(&mut self, output_mode: &mut OutputMode) {
        unsafe { ffi::wlr_output_state_set_mode(self.as_ptr(), output_mode.as_ptr()) };
    }

    #[doc = "Releases all resources associated with an output state."]
    pub fn finish(&mut self) {
        unsafe { ffi::wlr_output_state_finish(self.as_ptr()) };
    }
}

#[derive(FromPtr)]
pub struct OutputMode(ffi::wlr_output_mode);

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

    #[doc = "Add a viewport for the specified output to the scene-graph.\n\n An output can only be added once to the scene-graph."]
    pub fn output_create(&mut self, output: &mut Output) -> Result<SceneOutput, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_scene_output_create(self.as_ptr(), output.as_ptr()) })
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

#[derive(PtrWrapper)]
pub struct XdgShell(NonNull<ffi::wlr_xdg_shell>);

impl XdgShell {
    #[doc = "Create the xdg_wm_base global with the specified version."]
    pub fn create(display: &mut Display, version: u32) -> Result<XdgShell, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_xdg_shell_create(display.as_ptr(), version) })
            .map(Self)
            .ok_or_else(|| WrapperError::GeneralError("failed to create xdg shell".to_string()))
    }

    pub fn get_event_mut(&mut self, ty: XdgShellEvent) -> &mut Signal {
        use XdgShellEvent::*;
        let xdg_shell = unsafe { self.0.as_mut() };
        let event_ptr = match ty {
            NewSurface => &mut xdg_shell.events.new_surface,
            NewToplevel => &mut xdg_shell.events.new_toplevel,
            NewPopup => &mut xdg_shell.events.new_popup,
            Destroy => &mut xdg_shell.events.destroy,
        } as *mut ffi::wl_signal;
        let signal_ptr = event_ptr as *mut Signal;
        unsafe { &mut (*signal_ptr) as &mut Signal }
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
