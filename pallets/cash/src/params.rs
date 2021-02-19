use crate::symbol::{CASH, USD};
use crate::types::{Quantity, Timestamp};

// XXX do these belong in runtime interfaces config?

/// The number of blocks before an Ethereum transaction is considered final.
pub const ETH_FINALIZATION_BLOCKS: u32 = 30; // XXX

/// Number of milliseconds in a year.
pub const MILLISECONDS_PER_YEAR: Timestamp = 365 * 24 * 60 * 60 * 1000; // todo: xxx finalize this number

/// Minimum amount of time (milliseconds) into the future that a synchronized change may be scheduled for.
/// Must be sufficient time to propagate changes to L1s before they occur.
pub const MIN_NEXT_SYNC_TIME: u128 = 24 * 60 * 60 * 1000; // XXX

pub const MIN_VALIDATOR_COUNT: u32 = 4; // XXX needed? in the way of dev?

/// Minimum value (USD) required across all protocol interactions.
pub const MIN_TX_VALUE: Quantity = Quantity::from_nominal("1", USD);

/// Flat transfer fee (CASH).
pub const TRANSFER_FEE: Quantity = Quantity::from_nominal("0.01", CASH);

/// Number of blocks between HTTP requests from offchain workers to open oracle price feed.
pub const OCW_OPEN_ORACLE_POLL_INTERVAL_BLOCKS: u32 = 10;
