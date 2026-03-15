use std::error::Error;
use std::ffi::NulError;
use std::fmt::Display;
use std::str::Utf8Error;

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
    BackendStartFailure,
    GeneralError(String),
    NulError(NulError),
    Utf8Error(Utf8Error),
}

impl Display for WrapperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for WrapperError {}

macro_rules! from_error {
    ($($error:tt),* $(,)?) => {
        $(
            impl From<$error> for WrapperError {
                fn from(value: $error) -> Self {
                    Self::$error(value)
                }
            }
        )*
    };
}

from_error!(NulError, Utf8Error);
