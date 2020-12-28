#[cfg(feature = "std")]
mod std;
#[cfg(feature = "std")]
pub use crate::std::*;

mod no_std;
pub use no_std::*;
