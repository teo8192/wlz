use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub enum WrapperError {
    FailedToCreateDisplay,
    FailedToCreateRenderer,
    FailedToCreateBackend,
    FailedToCreateAllocator,
    FailedToCreateCompositor,
    FailedToCreateSubCompositor,
    FailedToCreateDataDeviceManager,
    FailedToCreateOutputLayout,
    FailedOutputLayoutAddAuto,
    FailedToCreateSceneOutput,
    FailedToCreateScene,
    FailedToAddSocket,
    FailedToInitializeDisplay,
    FailedSceneAttachOutputLayout,
    GeneralError(String),
}

impl Display for WrapperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for WrapperError {}
