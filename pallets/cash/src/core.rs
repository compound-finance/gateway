// Note: The substrate build requires these be re-exported.
pub use sp_std::{fmt, result, result::Result};

use crate::{chains, Reason};

pub fn apply_ethereum_event_internal(
    event: chains::EthereumEvent,
) -> Result<chains::EthereumEvent, Reason> {
    Ok(event) // XXX
}

#[cfg(test)]
mod tests {
    // XXX
}
