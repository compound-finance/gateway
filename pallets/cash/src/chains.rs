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

    // XXX Decode more fields and more event types
    pub fn decode(data: &[u8]) -> Event {
        let types = vec![
            ethabi::param_type::ParamType::Uint(256),
            ethabi::param_type::ParamType::Uint(256),
        ];
        let abi_decoded = ethabi::decode(&types[..], &data);
        let decoded = abi_decoded.unwrap();
        let block_number = ethereum_client::extract_uint(&decoded[0]).unwrap();
        let log_index = ethereum_client::extract_uint(&decoded[1]).unwrap();
        Event {
            id: (block_number.as_u32(), log_index.as_u32()),
        } // XXX
    }

    /// XXX Work on sending proper Payload,
    pub fn encode(event: &Event) -> Vec<u8> {
        let (block_number, log_index): (u32, u32) = event.id;
        ethabi::encode(&[
            ethabi::token::Token::Uint(block_number.into()),
            ethabi::token::Token::Uint(log_index.into()),
        ])
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
        sp_core::ecdsa::Signature::from_raw(r).encode()
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
