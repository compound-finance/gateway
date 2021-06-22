use crate::{
    chains::{ChainAccount, ChainBlockNumber},
    symbol::{CASH, USD},
    types::{CashPrincipal, Quantity, Timestamp},
};

/// The large value (USD) used for ingesting gov events.
pub const INGRESS_LARGE: Quantity = Quantity::from_nominal("1000000000000", USD);

/// The maximum value (USD) that can be ingested per underlying chain block.
/// Could become a per-chain quota in the future.
pub const INGRESS_QUOTA: Quantity = Quantity::from_nominal("10000", USD);

/// Maximum size of the block queue before we back-off sending new blocks.
pub const INGRESS_SLACK: u32 = 50;

/// Number of milliseconds in a year.
pub const MILLISECONDS_PER_YEAR: Timestamp = 365 * 24 * 60 * 60 * 1000;

/// Minimum number of underlying chain blocks to wait before ingesting any event, due to reorg risk.
pub const MIN_EVENT_BLOCKS: ChainBlockNumber = 3;

/// Maximum number of underlying chain blocks to wait before just ingesting any event.
pub const MAX_EVENT_BLOCKS: ChainBlockNumber = 60;

/// Minimum amount of time (milliseconds) into the future that a synchronized change may be scheduled for.
/// Must be sufficient time to propagate changes to L1s before they occur.
pub const MIN_NEXT_SYNC_TIME: Timestamp = 24 * 60 * 60 * 1000; // XXX confirm

/// Minimum CASH principal required in order to use a Gateway account.
/// Note that validators must meet this minimum in order to submit the set session keys extrinsic.
pub const MIN_PRINCIPAL_GATE: CashPrincipal = CashPrincipal::from_nominal("1");

/// Minimum value (USD) required across all protocol interactions.
pub const MIN_TX_VALUE: Quantity = Quantity::from_nominal("1", USD);

/// Flat transfer fee (CASH).
pub const TRANSFER_FEE: Quantity = Quantity::from_nominal("0.01", CASH);

/// The number of blocks in between periodic sessions.
pub const SESSION_PERIOD: u32 = 14400; // Assuming 6s blocks, ~1 period per day

/// Standard priority for all unsigned transactions.
pub const UNSIGNED_TXS_PRIORITY: u64 = 100;

/// Standard longevity for all unsigned transactions.
pub const UNSIGNED_TXS_LONGEVITY: u64 = 32;

/// Weight given to extrinsics that will exit early, to avoid spam.
pub const ERROR_WEIGHT: u64 = 100_000_000;

/// The void account from whence miner CASH is transferred out of.
pub const GATEWAY_VOID: ChainAccount = ChainAccount::Gate([0u8; 32]);
