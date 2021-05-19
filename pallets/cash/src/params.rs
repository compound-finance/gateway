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

/// Either testnet or stubnet definitions.
/// It's an error to flag more than one.
#[cfg(feature = "stubnet")]
pub use stubnet::*;
#[cfg(feature = "testnet")]
pub use testnet::*;

pub mod stubnet {
    use super::*;
    pub const ETH_STARPORT_ADDRESS: [u8; 20] = [0x77u8; 20];
    pub const ETH_STARPORT_BLOCK: ChainBlock = ChainBlock::Eth(ethereum_client::EthereumBlock {
        hash: [0x88u8; 32],
        parent_hash: [0x99u8; 32],
        number: 0xaabbccddeeff,
        events: vec![],
    });
}

pub mod testnet {
    use super::*;
    pub const ETH_STARPORT_ADDRESS: [u8; 20] = [
        217, 5, 171, 186, 28, 94, 164, 140, 5, 152, 190, 159, 63, 138, 227, 18, 144, 181, 134, 19,
    ];
    pub const ETH_STARPORT_BLOCK: ChainBlock = ChainBlock::Eth(ethereum_client::EthereumBlock {
        hash: [
            75, 122, 90, 123, 128, 75, 214, 240, 240, 192, 170, 80, 57, 45, 112, 27, 31, 242, 48,
            119, 13, 39, 213, 13, 14, 36, 14, 25, 70, 254, 135, 101,
        ],
        parent_hash: [
            175, 50, 222, 100, 197, 110, 190, 40, 194, 91, 128, 154, 223, 41, 229, 180, 78, 218,
            13, 144, 165, 133, 62, 215, 147, 158, 72, 252, 162, 241, 216, 218,
        ],
        number: 9853195,
        events: vec![],
    });
}
