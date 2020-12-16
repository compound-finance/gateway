// Note: The substrate build requires these be re-exported.
pub use our_std::{fmt, result, result::Result};

use crate::{chains, Reason};

pub fn apply_eth_event_internal(event: eth::Event) -> Result<eth::Event, Reason> {
    Ok(event) // XXX
}

#[cfg(test)]
mod tests {
    // XXX
}
