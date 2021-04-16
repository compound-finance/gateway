use crate::symbol::{CASH, USD};
use crate::types::{Quantity, Timestamp};

/// The large value (USD) used for ingesting gov events.
pub const INGRESS_LARGE: Quantity = Quantity::from_nominal("1000000000000", USD);

/// The maximum value (USD) that can be ingested per underlying chain block.
/// Could become a per-chain quota in the future.
pub const INGRESS_QUOTA: Quantity = Quantity::from_nominal("10000", USD);

/// Number of milliseconds in a year.
pub const MILLISECONDS_PER_YEAR: Timestamp = 365 * 24 * 60 * 60 * 1000;

/// Minimum amount of time (milliseconds) into the future that a synchronized change may be scheduled for.
/// Must be sufficient time to propagate changes to L1s before they occur.
pub const MIN_NEXT_SYNC_TIME: Timestamp = 24 * 60 * 60 * 1000; // XXX confirm

/// Minimum value (USD) required across all protocol interactions.
pub const MIN_TX_VALUE: Quantity = Quantity::from_nominal("1", USD);

/// Flat transfer fee (CASH).
pub const TRANSFER_FEE: Quantity = Quantity::from_nominal("0.01", CASH);

// The number of blocks in between periodic sessions
pub const SESSION_PERIOD: u32 = 14400; // Assuming 6s blocks, ~1 period per day

/// Standard priority for all unsigned transactions
/// More an be found here https://substrate.dev/docs/en/knowledgebase/learn-substrate/tx-pool
pub const UNSIGNED_TXS_PRIORITY: u64 = 100;

/// Standard longevity for all unsigned transactions
/// More an be found here https://substrate.dev/docs/en/knowledgebase/learn-substrate/tx-pool
pub const UNSIGNED_TXS_LONGEVITY: u64 = 32;
