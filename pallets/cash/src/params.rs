// XXX do these belong in runtime interfaces config?

/// The number of blocks before an Ethereum transaction is considered final.
pub const ETH_FINALIZATION_BLOCKS: u32 = 30; // XXX

/// Minimum amount of time (milliseconds) into the future that a synchronized change may be scheduled for.
/// Must be sufficient time to propagate changes to L1s before they occur.
pub const MIN_NEXT_SYNC_TIME: u32 = 24 * 60 * 60 * 1000; // XXX

pub const MIN_VALIDATOR_COUNT: u32 = 4; // XXX needed? in the way of dev?

/// Minimum value (USD) required across all protocol interactions.
pub const MIN_TX_VALUE: u128 = 1; // XXX how to represent $1?

/// Flat transfer fee (CASH).
pub const TRANSFER_FEE: f64 = 0.01; // XXX a CashAmount?