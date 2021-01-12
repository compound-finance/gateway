use crate::core::{Quantity, Symbol};

// XXX do these belong in runtime interfaces config?

/// The number of blocks before an Ethereum transaction is considered final.
pub const ETH_FINALIZATION_BLOCKS: u32 = 30; // XXX

/// Minimum amount of time (milliseconds) into the future that a synchronized change may be scheduled for.
/// Must be sufficient time to propagate changes to L1s before they occur.
pub const MIN_NEXT_SYNC_TIME: u32 = 24 * 60 * 60 * 1000; // XXX

pub const MIN_VALIDATOR_COUNT: u32 = 4; // XXX needed? in the way of dev?

/// Minimum value (USD) required across all protocol interactions.
pub const MIN_TX_VALUE: Quantity<{ Symbol::USD }> = Quantity::from_nominal(1.0);

/// Flat transfer fee (CASH).
pub const TRANSFER_FEE: Quantity<{ Symbol::CASH }> = Quantity::from_nominal(0.01);

/// Number of blocks between HTTP requests via offchain workers to open oracle price reporters
pub const OCW_OPEN_ORACLE_POLL_INTERVAL_BLOCKS: u32 = 10;
