// Note: The substrate build requires these be imported
pub use our_std::vec::Vec;

use crate::rates::APR;
use crate::types::{AssetAmount, MulIndex, Timestamp};
use codec::{Decode, Encode};
use our_std::{Debuggable, RuntimeDebug};

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainId {
    Comp,
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
    const ID: ChainId;

    type Address: Debuggable + Clone + Eq + Into<Vec<u8>> = [u8; 20];
    type Amount: Debuggable + Clone + Eq + Into<AssetAmount> = u128;
    type MulIndex: Debuggable + Clone + Eq + Into<MulIndex> = u128;
    type Rate: Debuggable + Clone + Eq + Into<APR> = u128;
    type Timestamp: Debuggable + Clone + Eq + Into<Timestamp> = u128; // XXX u64?
    type Hash: Debuggable + Clone + Eq = [u8; 32];
    type PublicKey: Debuggable + Clone + Eq = [u8; 64];
    type Signature: Debuggable + Clone + Eq = [u8; 65]; // XXX
    type EventId: Debuggable + Clone + Eq + Ord;
    type Event: Debuggable + Clone + Eq;

    fn hash_bytes(data: &[u8]) -> Self::Hash;
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Compound {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Ethereum {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Polkadot {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Solana {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Tezos {}

impl Chain for Compound {
    const ID: ChainId = ChainId::Comp;

    type EventId = comp::EventId;
    type Event = comp::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [0u8; 32] // XXX
    }
}

impl Chain for Ethereum {
    const ID: ChainId = ChainId::Eth;

    type EventId = eth::EventId;
    type Event = eth::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [0u8; 32] // XXX
    }
}

impl Chain for Polkadot {
    const ID: ChainId = ChainId::Dot;

    type EventId = dot::EventId;
    type Event = dot::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [1u8; 32] // XXX
    }
}

impl Chain for Solana {
    const ID: ChainId = ChainId::Sol;

    type EventId = sol::EventId;
    type Event = sol::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [2u8; 32] // XXX
    }
}

impl Chain for Tezos {
    const ID: ChainId = ChainId::Tez;

    type EventId = tez::EventId;
    type Event = tez::Event;

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        [3u8; 32] // XXX
    }
}

pub mod comp {
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    pub type EventId = (u64, u64); // XXX

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {}
}

pub mod eth {
    // Note: The substrate build requires these be imported
    pub use our_std::vec::Vec;

    use super::{Chain, Ethereum};
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;
    use tiny_keccak::Hasher;

    use crate::sig;

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub enum RecoveryError {
        SignatureRecoveryError,
    }

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
            yield_index: <Ethereum as Chain>::MulIndex,
        },

        Gov {
            // XXX all these become do?
        },
    }

    /// Helper function to quickly run keccak in the Ethereum-style
    /// TODO: Add to trait?
    pub fn digest(input: &[u8]) -> <Ethereum as Chain>::Hash {
        let mut output = [0u8; 32];
        let mut hasher = tiny_keccak::Keccak::v256();
        hasher.update(&input[..]);
        hasher.finalize(&mut output);
        output
    }

    /// Helper function to build address from public key
    /// TODO: Add to trait?
    pub fn address_from_public_key(
        public_key: <Ethereum as Chain>::PublicKey,
    ) -> <Ethereum as Chain>::Address {
        let mut address: [u8; 20] = [0; 20];
        let hash = digest(&public_key[..]);
        address.copy_from_slice(&hash[12..]);
        address
    }

    pub fn prepend_eth_signing_msg(message: &[u8]) -> Vec<u8> {
        let initial: Vec<u8> = b"\x19Ethereum Signed Message:\n".to_vec();
        let length: Vec<u8> = message.len().to_string().as_bytes().to_vec();
        let full_message: Vec<u8> = [initial, length, message.to_vec()].concat();

        full_message
    }

    // TODO: match by chain for signing algorithm or implement as trait
    pub fn sign(message: &[u8]) -> <Ethereum as Chain>::Signature {
        // TODO: get this from somewhere else
        let not_so_secret: [u8; 32] =
            hex_literal::hex!["50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374"];
        let private_key = secp256k1::SecretKey::parse(&not_so_secret).unwrap();
        let msg = secp256k1::Message::parse(&digest(&message));
        let x = secp256k1::sign(&msg, &private_key);

        let mut r: [u8; 65] = [0; 65];
        r[0..64].copy_from_slice(&x.0.serialize()[..]);
        r[64] = x.1.serialize() + 27; // TODO: 27 (Handle EIP-155)
        r
    }

    // TODO: Test this
    pub fn recover(
        message: &[u8],
        signature: <Ethereum as Chain>::Signature,
    ) -> Result<<Ethereum as Chain>::Address, RecoveryError> {
        let msg_digest = digest(message);

        let (rs, recovery_id, _chain_id) = sig::secp256k1_split_recovery_id(&signature)
            .map_err(|_| RecoveryError::SignatureRecoveryError)?;

        let pk = sig::secp256k1_recover(&msg_digest, &rs, recovery_id)
            .map_err(|_| RecoveryError::SignatureRecoveryError)?;
        Ok(address_from_public_key(pk))
    }
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
