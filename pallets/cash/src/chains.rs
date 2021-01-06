// Note: The substrate build requires these be imported
pub use our_std::vec::Vec;

use codec::{Decode, Encode};
use our_std::{Debuggable, RuntimeDebug};

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainId {
    Eth,
    Dot,
    Sol,
    Tez,
}

impl Default for ChainId {
    fn default() -> Self {
        ChainId::Eth
    }
}

pub trait Chain {
    type Address: Debuggable + Clone + Eq = [u8; 20];
    type Account: Debuggable + Clone + Eq = Self::Address;
    type Asset: Debuggable + Clone + Eq = Self::Address;
    type Amount: Debuggable + Clone + Eq = u128;
    type Index: Debuggable + Clone + Eq = u128;
    type Rate: Debuggable + Clone + Eq = u128;
    type Timestamp: Debuggable + Clone + Eq = u128;
    type Hash: Debuggable + Clone + Eq = [u8; 32];
    type PublicKey: Debuggable + Clone + Eq = [u8; 32];
    type Signature: Debuggable + Clone + Eq = [u8; 65]; // XXX
    type EventId: Debuggable + Clone + Eq + Ord;
    type Event: Debuggable + Clone + Eq;

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
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
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

    use super::{Chain, Ethereum};
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;
    use tiny_keccak::Hasher;

    pub type BlockNumber = u32;
    pub type LogIndex = u32;

    pub type EventId = (BlockNumber, LogIndex);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {
        pub id: EventId,
        pub data: EventData,
    }

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub enum EventData {
        // XXX only event is 'do'?
        Lock {
            asset: <Ethereum as Chain>::Address,
            holder: <Ethereum as Chain>::Address,
            amount: <Ethereum as Chain>::Amount,
        },

        LockCash {
            holder: <Ethereum as Chain>::Address,
            amount: <Ethereum as Chain>::Amount,
            yield_index: <Ethereum as Chain>::Amount,
        },

        Gov {
            // XXX all these become do?
        },
    }

    /// Helper function to quickly run keccak in the Ethereum-style
    pub fn keccak(input: Vec<u8>) -> <Ethereum as Chain>::Hash {
        let mut output = [0u8; 32];
        let mut hasher = tiny_keccak::Keccak::v256();
        hasher.update(&input[..]);
        hasher.finalize(&mut output);
        output
    }

    // TODO: match by chain for signing algorithm or implement as trait
    pub fn sign(message: &Vec<u8>) -> <Ethereum as Chain>::Signature {
        // TODO: get this from somewhere else
        let not_so_secret: [u8; 32] =
            hex_literal::hex!["50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374"];
        let private_key = secp256k1::SecretKey::parse(&not_so_secret).unwrap();

        let msg = secp256k1::Message::parse(&keccak(message.clone()));
        let x = secp256k1::sign(&msg, &private_key);

        let mut r: [u8; 65] = [0; 65];
        r[0..64].copy_from_slice(&x.0.serialize()[..]);
        r[64] = x.1.serialize();
        r
    }

    // XXX whats this?
    // pub fn to_eth_payload(notice: Notice) -> NoticePayload {
    //     let message = encode_ethereum_notice(notice);
    //     // TODO: do signer by chain
    //     let signer = "0x6a72a2f14577D9Cd0167801EFDd54a07B40d2b61"
    //         .as_bytes()
    //         .to_vec();
    //     NoticePayload {
    //         // id: move id,
    //         sig: sign(&message),
    //         msg: message.to_vec(), // perhaps hex::encode(message)
    //         signer: AccountIdent {
    //             chain: ChainIdent::Eth,
    //             account: signer,
    //         },
    //     }
    // }
}

pub mod dot {
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    pub type EventId = (u64, u64);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {}
}

pub mod sol {
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    pub type EventId = (u64, u64);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {}
}

pub mod tez {
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    pub type EventId = (u128, u128);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {}
}
