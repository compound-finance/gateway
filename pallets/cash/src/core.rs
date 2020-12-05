// Note: The substrate build requires these be re-exported.
pub use sp_std::{fmt, result, result::Result};

use crate::{chains::eth, Reason};

pub fn apply_ethereum_event_internal(event: eth::Event) -> Result<eth::Event, Reason> {
    Ok(event) // XXX
}

#[cfg(test)]
mod tests {
    // XXX
}
