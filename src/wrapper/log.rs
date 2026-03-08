use std::ffi::CString;

use crate::ffi;

pub enum LogLevel {
    Silent,
    Error,
    Info,
    Debug,
}

impl Into<ffi::wlr_log_importance> for LogLevel {
    fn into(self) -> ffi::wlr_log_importance {
        match self {
            LogLevel::Silent => ffi::wlr_log_importance_WLR_SILENT,
            LogLevel::Error => ffi::wlr_log_importance_WLR_ERROR,
            LogLevel::Info => ffi::wlr_log_importance_WLR_INFO,
            LogLevel::Debug => ffi::wlr_log_importance_WLR_DEBUG,
        }
    }
}

pub fn log_with_level(level: LogLevel, msg: &str) {
    let c_str = CString::new(msg).unwrap();
    unsafe { ffi::_wlr_log(level.into(), c_str.as_ptr()) };
}

pub fn init(level: LogLevel) {
    unsafe { ffi::wlr_log_init(level.into(), None) };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        $crate::wrapper::log::log_with_level(
            $crate::wrapper::log::LogLevel::Error,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::wrapper::log::log_with_level(
            $crate::wrapper::log::LogLevel::Info,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        $crate::wrapper::log::log_with_level(
            $crate::wrapper::log::LogLevel::Debug,
            &format!($($arg)*),
        );
    }};
}
