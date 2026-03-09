use std::mem::zeroed;
use std::ptr::{null_mut, NonNull};

use wlz_macros::{c_drop, FromPtr, PtrWrapper};

use super::wl::EventLoop;
use super::WrapperError;
use crate::ffi;
use crate::wrapper::wl::{Display, Signal};

#[derive(PtrWrapper)]
#[c_drop(ffi::wlr_backend_destroy)]
pub struct Backend(NonNull<ffi::wlr_backend>);

pub enum BackendEvent {
    Destroy,
    NewInput,
    NewOutput,
}

impl Backend {
    pub fn autocreate(event_loop: EventLoop) -> Result<Self, WrapperError> {
        /* The backend is a wlroots feature which abstracts the underlying input and
         * output hardware. The autocreate option will choose the most suitable
         * backend based on the current environment, such as opening an X11 window
         * if an X11 server is running. */
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

#[derive(PtrWrapper)]
#[c_drop(ffi::wlr_renderer_destroy)]
pub struct Renderer(NonNull<ffi::wlr_renderer>);

impl Renderer {
    pub fn autocreate(backend: &mut Backend) -> Result<Self, WrapperError> {
        /* Autocreates a renderer, either Pixman, GLES2 or Vulkan for us. The user
         * can also specify a renderer using the WLR_RENDERER env var.
         * The renderer is responsible for defining the various pixel formats it
         * supports for shared memory, this configures that for clients. */
        NonNull::new(unsafe { ffi::wlr_renderer_autocreate(backend.as_ptr()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateRenderer)
    }

    pub fn init_wl_display(&mut self, wl_display: &mut Display) -> Result<(), WrapperError> {
        if unsafe { ffi::wlr_renderer_init_wl_display(self.as_ptr(), wl_display.as_ptr()) } {
            Ok(())
        } else {
            Err(WrapperError::FailedToInitializeDisplay)
        }
    }
}

#[derive(PtrWrapper)]
#[c_drop(ffi::wlr_allocator_destroy)]
pub struct Allocator(NonNull<ffi::wlr_allocator>);

impl Allocator {
    pub fn autocreate(
        wlr_backend: &mut Backend,
        wlr_renderer: &mut Renderer,
    ) -> Result<Self, WrapperError> {
        /* Autocreates an allocator for us.
         * The allocator is the bridge between the renderer and the backend. It
         * handles the buffer creation, allowing wlroots to render onto the
         * screen */
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
    pub fn create(wl_display: &mut Display) -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_data_device_manager_create(wl_display.as_ptr()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateDataDeviceManager)
    }
}

#[derive(PtrWrapper)]
pub struct OutputLayout(NonNull<ffi::wlr_output_layout>);

impl OutputLayout {
    pub fn create(wl_display: &mut Display) -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_output_layout_create(wl_display.as_ptr()) })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateOutputLayout)
    }

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

#[derive(FromPtr)]
pub struct Output(ffi::wlr_output);

impl Output {
    pub fn init_renderer(&mut self, allocator: &mut Allocator, renderer: &mut Renderer) {
        // TODO: handle error
        unsafe {
            ffi::wlr_output_init_render(self.as_ptr(), allocator.as_ptr(), renderer.as_ptr())
        };
    }

    pub fn preferred_mode(&mut self) -> Option<&mut OutputMode> {
        let mode = unsafe { ffi::wlr_output_preferred_mode(self.as_ptr()) };

        NonNull::new(mode).map(|m| unsafe { OutputMode::from_ptr(m) })
    }

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

#[derive(FromPtr)]
pub struct OutputState(ffi::wlr_output_state);

impl OutputState {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut output_state = Self(unsafe { zeroed() });
        unsafe { ffi::wlr_output_state_init(output_state.as_ptr()) };

        output_state
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        unsafe { ffi::wlr_output_state_set_enabled(self.as_ptr(), enabled) };
    }

    pub fn set_mode(&mut self, output_mode: &mut OutputMode) {
        unsafe { ffi::wlr_output_state_set_mode(self.as_ptr(), output_mode.as_ptr()) };
    }

    pub fn finish(&mut self) {
        unsafe { ffi::wlr_output_state_finish(self.as_ptr()) };
    }
}

#[derive(FromPtr)]
pub struct OutputMode(ffi::wlr_output_mode);

#[derive(PtrWrapper)]
pub struct SceneOutputLayout(NonNull<ffi::wlr_scene_output_layout>);

impl SceneOutputLayout {
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

#[derive(PtrWrapper)]
pub struct SceneOutput(NonNull<ffi::wlr_scene_output>);

#[derive(PtrWrapper)]
pub struct Scene(NonNull<ffi::wlr_scene>);

impl Scene {
    pub fn create() -> Result<Self, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_scene_create() })
            .map(Self)
            .ok_or(WrapperError::FailedToCreateScene)
    }

    pub fn output_create(&mut self, output: &mut Output) -> Result<SceneOutput, WrapperError> {
        NonNull::new(unsafe { ffi::wlr_scene_output_create(self.as_ptr(), output.as_ptr()) })
            .map(SceneOutput)
            .ok_or(WrapperError::FailedToCreateSceneOutput)
    }

    pub fn attach_output_layout(
        &mut self,
        output_layout: &mut OutputLayout,
    ) -> Result<SceneOutputLayout, WrapperError> {
        NonNull::new(unsafe {
            ffi::wlr_scene_attach_output_layout(self.as_ptr(), output_layout.as_ptr())
        })
        .map(SceneOutputLayout)
        .ok_or(WrapperError::FailedSceneAttachOutputLayout)
    }
}
