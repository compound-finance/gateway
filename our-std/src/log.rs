#![macro_use]

pub use log;

// This file includes macros to make it easy to log from either an
// std or no_std environment. Just use log!("My Log: {}", 5); and
// things should just magically work.

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        $crate::log::log::info!($($arg)*);
    }}
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        $crate::log::log::debug!($($arg)*);
    }}
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{
        $crate::log::log::trace!($($arg)*);
    }}
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::log::log::info!($($arg)*);
    }}
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        $crate::log::log::warn!($($arg)*);
    }}
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        $crate::log::log::error!($($arg)*);
    }}
}
