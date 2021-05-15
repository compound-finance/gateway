use crate::{
    chains::{ChainBlock, ChainBlockNumber},
    symbol::{CASH, USD},
    types::{Quantity, Timestamp},
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

// Weight given to extrinsics that will exit early, to avoid spam
pub const ERROR_WEIGHT: u64 = 100_000_000;

/// The Ethereum Starport address (to be replaced).
pub const ETH_STARPORT_ADDRESS: [u8; 20] = [237, 60, 225, 174, 49, 97, 172, 143, 135, 193, 207, 130, 242, 169, 114, 226, 235, 45, 211, 173]; // XXX

/// The Ethereum Starport last block info (each to be replaced).
pub const ETH_STARPORT_BLOCK: ChainBlock = ChainBlock::Eth(
    ethereum_client::EthereumBlock {
        // ETH_STARPORT_BLOCK_HASH
        hash: [89, 27, 30, 104, 20, 177, 24, 56, 61, 76, 166, 51, 20, 153, 110, 183, 61, 146, 233, 239, 24, 70, 145, 98, 20, 51, 118, 181, 83, 28, 217, 17],
        // ETH_STARPORT_BLOCK_PARENT_HASH
        parent_hash: [185, 86, 181, 91, 5, 124, 81, 41, 80, 95, 188, 208, 14, 175, 229, 236, 244, 27, 246, 41, 255, 142, 229, 83, 126, 253, 254, 16, 9, 124, 76, 164],
        // ETH_STARPORT_BLOCK_NUMBER
        number: 15015590786536052272,
        events: vec![],
    }); // XXX
