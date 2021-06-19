#![feature(const_panic)]
//! Makes available the things we use from Substrate, no matter std or no-std.

// The ones substrate takes care of already.
pub use sp_std::alloc;
pub use sp_std::any;
pub use sp_std::borrow;
pub use sp_std::boxed;
pub use sp_std::cell;
pub use sp_std::clone;
pub use sp_std::cmp;
pub use sp_std::convert;
pub use sp_std::default;
pub use sp_std::fmt;
pub use sp_std::hash;
pub use sp_std::if_std;
pub use sp_std::iter;
pub use sp_std::marker;
pub use sp_std::mem;
pub use sp_std::num;
pub use sp_std::ops;
pub use sp_std::ptr;
pub use sp_std::rc;
pub use sp_std::result;
pub use sp_std::slice;
pub use sp_std::str;
pub use sp_std::vec;

pub mod collections {
    pub use sp_std::collections::btree_map;
    pub use sp_std::collections::btree_set;
    pub use sp_std::collections::vec_deque;
}

pub mod thread {
    pub use sp_std::thread::panicking;
}

pub mod consts;
pub mod fixed_width;
pub mod log;

// The ones it doesn't.
#[cfg(feature = "std")]
include!("../with_std.rs");
#[cfg(not(feature = "std"))]
include!("../without_std.rs");
