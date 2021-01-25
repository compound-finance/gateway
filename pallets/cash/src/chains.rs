// Note: The substrate build requires these be imported
pub use our_std::vec::Vec;

use crate::types::{AssetAmount, MulIndex, Timestamp, APR};

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
    use compound_crypto::CryptoError;
    use our_std::convert::TryInto;
    use our_std::RuntimeDebug;
    use tiny_keccak::Hasher;

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

    /// Sign messages for the ethereum network
    pub fn sign(
        messages: Vec<&[u8]>,
    ) -> Result<Vec<Result<<Ethereum as Chain>::Signature, CryptoError>>, CryptoError> {
        // TODO: match by chain for signing algorithm or implement as trait
        let eth_key_id = runtime_interfaces::validator_config_interface::get_eth_key_id()
            .ok_or(CryptoError::KeyNotFound)?;
        // need to materialize this to pass over runtime interface boundary
        let messages: Vec<Vec<u8>> = messages
            .iter()
            .map(|e| e.iter().map(|f| *f).collect())
            .collect();

        runtime_interfaces::keyring_interface::sign(messages, eth_key_id)
    }

    /// Sign messages for the ethereum network
    pub fn sign_one(message: &[u8]) -> Result<<Ethereum as Chain>::Signature, CryptoError> {
        let message = Vec::from(message);
        let eth_key_id = runtime_interfaces::validator_config_interface::get_eth_key_id()
            .ok_or(CryptoError::KeyNotFound)?;
        runtime_interfaces::keyring_interface::sign_one(message, eth_key_id)
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
