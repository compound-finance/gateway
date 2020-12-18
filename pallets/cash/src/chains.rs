// Note: The substrate build requires these be imported
pub use our_std::vec::Vec;

use codec::{Decode, Encode};
use our_std::{Debuggable, Deserialize, RuntimeDebug, Serialize};

// XXX where should this live? with e.g. ethereum_client?
pub mod eth {
    // Note: The substrate build requires these be imported
    pub use our_std::vec::Vec;

    pub type BlockNumber = u32;
    pub type LogIndex = u32;
    pub type EventId = (BlockNumber, LogIndex);

    #[derive(Clone, Copy)]
    pub struct Event {
        pub id: EventId,
    }

    pub fn decode(data: &[u8]) -> Event {
        Event { id: (13, 37) } // XXX
    }

    /// XXX is Decoding and encoding useless here
    pub fn encode(event: &Event) -> Vec<u8> {
        let (block_number, log_index): (u32, u32) = event.id;
        ethabi::encode(&[
            ethabi::token::Token::Int(block_number.into()),
            ethabi::token::Token::Int(log_index.into()),
        ])
    }
}

pub type EraId = u32;
pub type EraIndex = u32;
pub type NoticeId = (EraId, EraIndex);

pub trait L1 {
    type Address: Debuggable = [u8; 20];
    type Account: Debuggable = Self::Address;
    type Asset: Debuggable = Self::Address;
    type Amount: Debuggable = u128;
    type Index: Debuggable = u128;
    type Rate: Debuggable = u128;
    type Timestamp: Debuggable = u128;
    type Hash: Debuggable = [u8; 32];
    type Public: Debuggable = [u8; 32];

    fn hash_bytes(data: &[u8]) -> Self::Hash;
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Ethereum {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Polkadot {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Solana {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Tezos {}

impl L1 for Ethereum {
    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [0u8; 32] // XXX
    }
}

impl L1 for Polkadot {
    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [1u8; 32] // XXX
    }
}

impl L1 for Solana {
    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [2u8; 32] // XXX
    }
}

impl L1 for Tezos {
    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [3u8; 32] // XXX
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize, RuntimeDebug)]
pub enum EventStatus<Chain: L1> {
    Pending {
        signers: Vec<u8>,
    }, // XXX set(Public)?
    Failed {
        hash: Chain::Hash,
        reason: crate::Reason,
    }, // XXX type for err reasons?
    Done,
}
