#![macro_use]

pub use log;

// This file includes macros to make it easy to log from either an
// std or no_std environment. Just use log!("My Log: {}", 5); and
// things should just magically work.

#[cfg(feature = "std")]
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        $crate::log::log::info!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        sp_runtime::print($crate::alloc::format!($($arg)*).as_str());
    }}
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        $crate::log::log::debug!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        $crate::log!($($arg)*);
    }}
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::log::log::info!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::log!($($arg)*);
    }}
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        $crate::log::log::warn!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        $crate::log!($($arg)*);
    }}
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        $crate::log::log::error!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        $crate::log!($($arg)*);
    }}
}
