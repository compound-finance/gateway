pub use our_std_proc_macro::{Deserialize, Serialize};
pub use sp_runtime::RuntimeDebug;
pub use core::fmt::Debug as Debuggable;

pub trait Serialize {}
pub trait Deserialize<'de> {}
