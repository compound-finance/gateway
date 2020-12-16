// Note: The substrate build requires these be imported
pub use sp_std::vec::Vec;

// XXX where should this live? with e.g. ethereum_client?
pub mod eth {
    // Note: The substrate build requires these be imported
    pub use sp_std::vec::Vec;

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

// XXX can we elim Debug / bound on our own catch all?
pub trait L1 {
    type Address: std::fmt::Debug = [u8; 20];
    type Account: std::fmt::Debug = Self::Address;
    type Asset: std::fmt::Debug = Self::Address;
    type Amount: std::fmt::Debug = u128;
    type Index: std::fmt::Debug = u128;
    type Rate: std::fmt::Debug = u128;
    type Timestamp: std::fmt::Debug = u128;
    type Hash: std::fmt::Debug = [u8; 32];
    type Public: std::fmt::Debug = [u8; 32];

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

// XXX better way to use these?
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Notice<Chain: L1> {
    ExtractionNotice {
        id: NoticeId,
        parent: Chain::Hash,
        asset: Chain::Asset,
        account: Chain::Account,
        amount: Chain::Amount,
    },

    CashExtractionNotice {
        id: NoticeId,
        parent: Chain::Hash,
        account: Chain::Account,
        amount: Chain::Amount,
        cash_yield_index: Chain::Index,
    },

    FutureYieldNotice {
        id: NoticeId,
        parent: Chain::Hash,
        next_cash_yield: Chain::Rate,
        next_cash_yield_start_at: Chain::Timestamp,
        next_cash_yield_index: Chain::Index,
    },

    SetSupplyCapNotice {
        id: NoticeId,
        parent: Chain::Hash,
        asset: Chain::Asset,
        amount: Chain::Amount,
    },

    ChangeAuthorityNotice {
        id: NoticeId,
        parent: Chain::Hash,
        new_authorities: Vec<Chain::Public>,
    },
}

impl<Chain: L1> Notice<Chain> {
    pub fn id(&self) -> NoticeId {
        match self {
            Notice::ExtractionNotice { id, .. } => *id,
            Notice::CashExtractionNotice { id, .. } => *id,
            Notice::FutureYieldNotice { id, .. } => *id,
            Notice::SetSupplyCapNotice { id, .. } => *id,
            Notice::ChangeAuthorityNotice { id, .. } => *id,
        }
    }
}
