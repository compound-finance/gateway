// Note: The substrate build requires these be imported
pub use our_std::vec::Vec;

use codec::{Decode, Encode};
use our_std::{Debuggable, Deserialize, RuntimeDebug, Serialize};

#[derive(Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize, RuntimeDebug)]
pub enum ChainId {
    Eth,
    Dot,
    Sol,
    Tez,
}

pub trait Chain {
    type Address: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = [u8; 20];
    type Account: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = Self::Address;
    type Asset: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = Self::Address;
    type Amount: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = u128;
    type Index: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = u128;
    type Rate: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = u128;
    type Timestamp: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = u128;
    type Hash: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = [u8; 32];
    type PublicKey: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = [u8; 32];
    type Signature: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a> = [u8; 32];
    type EventId: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a>; // XXX make totally ordered trait
    type Event: Debuggable + Clone + Eq + Serialize + for<'a> Deserialize<'a>;

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

impl Chain for Ethereum {
    type EventId = eth::EventId;
    type Event = eth::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [0u8; 32] // XXX
    }
}

impl Chain for Polkadot {
    type EventId = dot::EventId;
    type Event = dot::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [1u8; 32] // XXX
    }
}

impl Chain for Solana {
    type EventId = sol::EventId;
    type Event = sol::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [2u8; 32] // XXX
    }
}

impl Chain for Tezos {
    type EventId = tez::EventId;
    type Event = tez::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [3u8; 32] // XXX
    }
}

// XXX move?
#[derive(Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize, RuntimeDebug)]
pub enum EventStatus<C: Chain> {
    Pending {
        signers: crate::ValidatorSet,
    },
    Failed {
        hash: C::Hash,
        reason: crate::Reason,
    },
    Done,
}

pub mod eth {
    // Note: The substrate build requires these be imported
    pub use our_std::vec::Vec;

    use codec::{Decode, Encode};
    use our_std::{Deserialize, RuntimeDebug, Serialize};

    pub type BlockNumber = u32;
    pub type LogIndex = u32;
    pub type EventId = (BlockNumber, LogIndex);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize, RuntimeDebug)]
    pub struct Event {
        pub id: EventId,
    }

    // XXX kill these?
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

pub mod dot {
    use codec::{Decode, Encode};
    use our_std::{Deserialize, RuntimeDebug, Serialize};

    pub type EventId = (u64, u64);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize, RuntimeDebug)]
    pub struct Event {}
}

pub mod sol {
    use codec::{Decode, Encode};
    use our_std::{Deserialize, RuntimeDebug, Serialize};

    pub type EventId = (u64, u64);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize, RuntimeDebug)]
    pub struct Event {}
}

pub mod tez {
    use codec::{Decode, Encode};
    use our_std::{Deserialize, RuntimeDebug, Serialize};

    pub type EventId = (u128, u128);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize, RuntimeDebug)]
    pub struct Event {}
}
