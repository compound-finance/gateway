#![macro_use]

// This file includes macros to make it easy to log from either an
// std or no_std environment. Just use log!("My Log: {}", 5); and
// things should just magically work.

#[cfg(feature = "std")]
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        frame_support::debug::native::info!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        sp_runtime::print(format!($($arg)*).as_str());
    }}
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        frame_support::debug::native::debug!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        log!($($arg)*);
    }}
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        frame_support::debug::native::info!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        log!($($arg)*);
    }}
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        frame_support::debug::native::warn!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        log!($($arg)*);
    }}
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        frame_support::debug::native::error!($($arg)*);
    }}
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        log!($($arg)*);
    }}
}
