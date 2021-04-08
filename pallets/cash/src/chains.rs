// Note: The substrate build requires these be imported
pub use our_std::vec::Vec;

use crate::rates::APR;
use crate::reason::Reason;
use crate::types::{AssetAmount, CashIndex, SignersSet, Timestamp, ValidatorIdentity};

use codec::{Decode, Encode};
use gateway_crypto::public_key_bytes_to_eth_address;
use our_std::{str::FromStr, vec, Debuggable, Deserialize, RuntimeDebug, Serialize};

use types_derive::{type_alias, Types};

/// Type for representing the selection of a supported chain.
#[derive(Serialize, Deserialize)] // used in config
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainId {
    Gate,
    Eth,
    Dot,
    Sol,
    Tez,
}

impl ChainId {
    pub fn to_account(self, addr: &str) -> Result<ChainAccount, Reason> {
        match self {
            ChainId::Gate => Ok(ChainAccount::Gate(Gateway::str_to_address(addr)?)),
            ChainId::Eth => Ok(ChainAccount::Eth(Ethereum::str_to_address(addr)?)),
            ChainId::Dot => Ok(ChainAccount::Dot(Polkadot::str_to_address(addr)?)),
            ChainId::Sol => Ok(ChainAccount::Sol(Solana::str_to_address(addr)?)),
            ChainId::Tez => Ok(ChainAccount::Tez(Tezos::str_to_address(addr)?)),
        }
    }

    pub fn to_asset(self, addr: &str) -> Result<ChainAsset, Reason> {
        match self {
            ChainId::Gate => Ok(ChainAsset::Gate(Gateway::str_to_address(addr)?)),
            ChainId::Eth => Ok(ChainAsset::Eth(Ethereum::str_to_address(addr)?)),
            ChainId::Dot => Ok(ChainAsset::Dot(Polkadot::str_to_address(addr)?)),
            ChainId::Sol => Ok(ChainAsset::Sol(Solana::str_to_address(addr)?)),
            ChainId::Tez => Ok(ChainAsset::Tez(Tezos::str_to_address(addr)?)),
        }
    }

    pub fn signer_address(self) -> Result<ChainAccount, Reason> {
        match self {
            ChainId::Gate => Ok(ChainAccount::Gate(<Gateway as Chain>::signer_address()?)),
            ChainId::Eth => Ok(ChainAccount::Eth(<Ethereum as Chain>::signer_address()?)),
            ChainId::Dot => Ok(ChainAccount::Dot(<Polkadot as Chain>::signer_address()?)),
            ChainId::Sol => Ok(ChainAccount::Sol(<Solana as Chain>::signer_address()?)),
            ChainId::Tez => Ok(ChainAccount::Tez(<Tezos as Chain>::signer_address()?)),
        }
    }

    pub fn hash_bytes(self, data: &[u8]) -> ChainHash {
        match self {
            ChainId::Gate => ChainHash::Gate(<Gateway as Chain>::hash_bytes(data)),
            ChainId::Eth => ChainHash::Eth(<Ethereum as Chain>::hash_bytes(data)),
            ChainId::Dot => ChainHash::Dot(<Polkadot as Chain>::hash_bytes(data)),
            ChainId::Sol => ChainHash::Sol(<Solana as Chain>::hash_bytes(data)),
            ChainId::Tez => ChainHash::Tez(<Tezos as Chain>::hash_bytes(data)),
        }
    }

    pub fn sign(self, message: &[u8]) -> Result<ChainSignature, Reason> {
        match self {
            ChainId::Gate => Ok(ChainSignature::Gate(<Gateway as Chain>::sign_message(
                message,
            )?)),
            ChainId::Eth => Ok(ChainSignature::Eth(<Ethereum as Chain>::sign_message(
                message,
            )?)),
            ChainId::Dot => Ok(ChainSignature::Dot(<Polkadot as Chain>::sign_message(
                message,
            )?)),
            ChainId::Sol => Ok(ChainSignature::Sol(<Solana as Chain>::sign_message(
                message,
            )?)),
            ChainId::Tez => Ok(ChainSignature::Tez(<Tezos as Chain>::sign_message(
                message,
            )?)),
        }
    }

    pub fn zero_hash(self) -> ChainHash {
        match self {
            ChainId::Gate => ChainHash::Gate(<Gateway as Chain>::zero_hash()),
            ChainId::Eth => ChainHash::Eth(<Ethereum as Chain>::zero_hash()),
            ChainId::Dot => ChainHash::Dot(<Polkadot as Chain>::zero_hash()),
            ChainId::Sol => ChainHash::Sol(<Solana as Chain>::zero_hash()),
            ChainId::Tez => ChainHash::Tez(<Tezos as Chain>::zero_hash()),
        }
    }
}

// XXX why?
impl Default for ChainId {
    fn default() -> Self {
        ChainId::Eth
    }
}

/// Type for an account tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainAccount {
    Gate(<Gateway as Chain>::Address),
    Eth(<Ethereum as Chain>::Address),
    Dot(<Polkadot as Chain>::Address),
    Sol(<Solana as Chain>::Address),
    Tez(<Tezos as Chain>::Address),
}

impl ChainAccount {
    pub fn chain_id(&self) -> ChainId {
        match *self {
            ChainAccount::Eth(_) => ChainId::Eth,
            _ => panic!("XXX not implemented"),
        }
    }
}

// Implement deserialization for ChainAccounts so we can use them in GenesisConfig / ChainSpec JSON.
//  i.e. "eth:0x..." <> Eth(0x...)
impl FromStr for ChainAccount {
    type Err = Reason;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if let Some((chain_id_str, address_str)) = String::from(string).split_once(":") {
            let chain_id = ChainId::from_str(chain_id_str)?;
            Ok(chain_id.to_account(address_str)?)
        } else {
            Err(Reason::BadAsset)
        }
    }
}

// For serialize (which we don't really use, but are required to implement)
impl From<ChainAccount> for String {
    fn from(asset: ChainAccount) -> String {
        match asset {
            ChainAccount::Eth(address) => format!("ETH:0x{}", hex::encode(address)),
            _ => panic!("XXX not implemented"),
        }
    }
}

/// Type for an asset tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainAsset {
    Gate(<Gateway as Chain>::Address),
    Eth(<Ethereum as Chain>::Address),
    Dot(<Polkadot as Chain>::Address),
    Sol(<Solana as Chain>::Address),
    Tez(<Tezos as Chain>::Address),
}

// For serialize (which we don't really use, but are required to implement)
impl ChainAsset {
    pub fn chain_id(&self) -> ChainId {
        match *self {
            ChainAsset::Eth(_) => ChainId::Eth,
            _ => panic!("XXX not implemented"),
        }
    }
}

// Implement deserialization for ChainAssets so we can use them in GenesisConfig / ChainSpec JSON.
//  i.e. "eth:0x..." <> Eth(0x...)
impl FromStr for ChainAsset {
    type Err = Reason;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if let Some((chain_id_str, address_str)) = String::from(string).split_once(":") {
            let chain_id = ChainId::from_str(chain_id_str)?;
            Ok(chain_id.to_asset(address_str)?)
        } else {
            Err(Reason::BadAsset)
        }
    }
}

impl From<ChainAsset> for String {
    fn from(asset: ChainAsset) -> String {
        match asset {
            ChainAsset::Eth(address) => format!("ETH:0x{}", hex::encode(address)),
            _ => panic!("XXX not implemented"),
        }
    }
}

/// Type for a signature and account tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainAccountSignature {
    Gate(<Gateway as Chain>::Address, <Gateway as Chain>::Signature),
    Eth(<Ethereum as Chain>::Address, <Ethereum as Chain>::Signature),
    Dot(<Polkadot as Chain>::Address, <Polkadot as Chain>::Signature),
    Sol(<Solana as Chain>::Address, <Solana as Chain>::Signature),
    Tez(<Tezos as Chain>::Address, <Tezos as Chain>::Signature),
}

impl ChainAccountSignature {
    pub fn to_chain_signature(self) -> ChainSignature {
        match self {
            ChainAccountSignature::Gate(_, sig) => ChainSignature::Gate(sig),
            ChainAccountSignature::Eth(_, sig) => ChainSignature::Eth(sig),
            ChainAccountSignature::Dot(_, sig) => ChainSignature::Dot(sig),
            ChainAccountSignature::Sol(_, sig) => ChainSignature::Sol(sig),
            ChainAccountSignature::Tez(_, sig) => ChainSignature::Tez(sig),
        }
    }

    pub fn recover_account(self, message: &[u8]) -> Result<ChainAccount, Reason> {
        match self {
            ChainAccountSignature::Eth(eth_account, eth_sig) => {
                let recovered = <Ethereum as Chain>::recover_user_address(message, eth_sig)?;
                if eth_account == recovered {
                    Ok(ChainAccount::Eth(recovered))
                } else {
                    Err(Reason::SignatureAccountMismatch)
                }
            }
            _ => panic!("XXX not implemented"),
        }
    }
}

/// Type for a block number tied on an underlying chain.
pub type ChainBlockNumber = u64;

/// Type for an hash tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainHash {
    Gate(<Gateway as Chain>::Hash),
    Eth(<Ethereum as Chain>::Hash),
    Dot(<Polkadot as Chain>::Hash),
    Sol(<Solana as Chain>::Hash),
    Tez(<Tezos as Chain>::Hash),
}

/// Type for a signature tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainSignature {
    Gate(<Gateway as Chain>::Signature),
    Eth(<Ethereum as Chain>::Signature),
    Dot(<Polkadot as Chain>::Signature),
    Sol(<Solana as Chain>::Signature),
    Tez(<Tezos as Chain>::Signature),
}

impl ChainSignature {
    pub fn chain_id(&self) -> ChainId {
        match *self {
            ChainSignature::Eth(_) => ChainId::Eth,
            _ => panic!("XXX not implemented"),
        }
    }

    pub fn recover(&self, message: &[u8]) -> Result<ChainAccount, Reason> {
        match self {
            ChainSignature::Eth(eth_sig) => Ok(ChainAccount::Eth(
                <Ethereum as Chain>::recover_address(message, *eth_sig)?,
            )),

            _ => panic!("XXX not implemented"),
        }
    }
}

/// Type for a list of chain signatures.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainSignatureList {
    Gate(Vec<(<Gateway as Chain>::Address, <Gateway as Chain>::Signature)>),
    Eth(Vec<(<Ethereum as Chain>::Address, <Ethereum as Chain>::Signature)>),
    Dot(Vec<(<Polkadot as Chain>::Address, <Polkadot as Chain>::Signature)>),
    Sol(Vec<(<Solana as Chain>::Address, <Solana as Chain>::Signature)>),
    Tez(Vec<(<Tezos as Chain>::Address, <Tezos as Chain>::Signature)>),
}

// Implement deserialization for ChainIds so we can use them in GenesisConfig / ChainSpec JSON.
impl FromStr for ChainId {
    type Err = Reason;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "ETH" => Ok(ChainId::Eth),
            "SOL" => Ok(ChainId::Sol),
            _ => Err(Reason::BadChainId),
        }
    }
}

/// Type for describing a block coming from an underlying chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainBlock {
    Eth(<Ethereum as Chain>::Block),
}

impl ChainBlock {
    pub fn chain_id(&self) -> ChainId {
        match self {
            ChainBlock::Eth(_) => ChainId::Eth,
        }
    }

    pub fn hash(&self) -> ChainHash {
        match self {
            ChainBlock::Eth(block) => ChainHash::Eth(block.hash),
        }
    }

    pub fn parent_hash(&self) -> ChainHash {
        match self {
            ChainBlock::Eth(block) => ChainHash::Eth(block.parent_hash),
        }
    }

    pub fn number(&self) -> ChainBlockNumber {
        match self {
            ChainBlock::Eth(block) => block.number as ChainBlockNumber,
        }
    }
}

/// Type for describing a set of blocks coming from an underlying chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainBlocks {
    Eth(Vec<<Ethereum as Chain>::Block>),
}

impl ChainBlocks {
    pub fn chain_id(&self) -> ChainId {
        match *self {
            ChainBlocks::Eth(_) => ChainId::Eth,
        }
    }
}

impl IntoIterator for ChainBlocks {
    type Item = ChainBlock;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            // XXX can we avoid collect without knowing inner type of (map function for iter::Map)?
            ChainBlocks::Eth(blocks) => blocks
                .into_iter()
                .map(|b| ChainBlock::Eth(b))
                .collect::<Vec<ChainBlock>>()
                .into_iter(),
        }
    }
}

/// Type for describing a reorg of an underlying chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainReorg {
    Eth {
        from_hash: <Ethereum as Chain>::Hash,
        to_hash: <Ethereum as Chain>::Hash,
        reverse_blocks: Vec<<Ethereum as Chain>::Block>,
        forward_blocks: Vec<<Ethereum as Chain>::Block>,
    },
}

impl ChainReorg {
    pub fn chain_id(&self) -> ChainId {
        match *self {
            ChainReorg::Eth { .. } => ChainId::Eth,
        }
    }

    pub fn from_hash(&self) -> ChainHash {
        match *self {
            ChainReorg::Eth { from_hash, .. } => ChainHash::Eth(from_hash),
        }
    }
}

// XXX old code
// pub fn passes_validation_threshold(
//     signers: &BTreeSet<ValidatorIdentity>,
//     validators: &BTreeSet<ValidatorIdentity>,
// ) -> bool {
//     // Intersection is taken for the situation when some of the signers are not currently active validators
//     let valid_signers: Vec<_> = validators.intersection(&signers).collect();
//     // Using ceil(2 * validators.len() / 3)
//     valid_signers.len() >= (2 * validators.len() + 3 - 1) / 3
// }

/// Type for tallying signatures for an underlying chain block.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct ChainBlockTally {
    pub block: ChainBlock,
    pub for_votes: SignersSet,
    pub against_votes: SignersSet,
}

impl ChainBlockTally {
    pub fn new(
        chain_id: ChainId,
        block: ChainBlock,
        validator: ValidatorIdentity,
    ) -> ChainBlockTally {
        match chain_id {
            ChainId::Eth => ChainBlockTally {
                block,
                for_votes: [validator].iter().cloned().collect(),
                against_votes: SignersSet::new(),
            },

            _ => panic!("xxx not implemented"),
        }
    }

    pub fn passes_threshold(self) -> bool {
        false // XXX also need to check internally? use this or different?
    }

    // XXX delete?
    // XXX for or against?
    pub fn with_signer(self, signature: ValidatorIdentity) -> Self {
        self
    }
}

/// Type for tallying signatures for an underlying chain reorg.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct ChainReorgTally {
    pub reorg: ChainReorg,
    pub votes: SignersSet,
}

impl ChainReorgTally {
    pub fn new(
        chain_id: ChainId,
        reorg: ChainReorg,
        validator: ValidatorIdentity,
    ) -> ChainReorgTally {
        match chain_id {
            ChainId::Eth => ChainReorgTally {
                reorg,
                votes: [validator].iter().cloned().collect(),
            },

            _ => panic!("xxx not implemented"),
        }
    }

    pub fn passes_threshold(self) -> bool {
        false // XXX also need to check internally? use this or different?
    }

    // XXX takes identity...
    pub fn with_signer(self, signer: ValidatorIdentity) -> Self {
        self
    }
}

/// Type for describing a set of events coming from an underlying chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainEvents {
    Eth(Vec<<Ethereum as Chain>::Event>),
}

impl ChainEvents {
    pub fn push(&mut self, block: &ChainBlock) {
        // XXX add block to queue in place
        match self {
            ChainEvents::Eth(eth_events) => match block {
                ChainBlock::Eth(eth_block) => eth_events.extend_from_slice(&eth_block.events),
            },
        }
    }
}

pub trait Chain {
    const ID: ChainId;

    type Address: Debuggable + Clone + Eq + Into<Vec<u8>>;
    type Amount: Debuggable + Clone + Eq + Into<AssetAmount>;
    type CashIndex: Debuggable + Clone + Eq + Into<CashIndex>;
    type Rate: Debuggable + Clone + Eq + Into<APR>;
    type Timestamp: Debuggable + Clone + Eq + Into<Timestamp>;
    type Hash: Debuggable + Clone + Eq;
    type PublicKey: Debuggable + Clone + Eq;
    type Signature: Debuggable + Clone + Eq;
    type EventId: Debuggable + Clone + Eq + Ord;
    type Event: Debuggable + Clone + Eq;
    type Block: Debuggable + Clone + Eq;

    fn zero_hash() -> Self::Hash;
    fn hash_bytes(data: &[u8]) -> Self::Hash;
    fn recover_user_address(
        data: &[u8],
        signature: Self::Signature,
    ) -> Result<Self::Address, Reason>;
    fn recover_address(data: &[u8], signature: Self::Signature) -> Result<Self::Address, Reason>;
    fn sign_message(message: &[u8]) -> Result<Self::Signature, Reason>;
    fn signer_address() -> Result<Self::Address, Reason>;
    fn str_to_address(addr: &str) -> Result<Self::Address, Reason>;
    fn address_string(address: &Self::Address) -> String;
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Gateway {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Ethereum {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Polkadot {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Solana {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Tezos {}

impl Chain for Gateway {
    const ID: ChainId = ChainId::Gate;

    #[type_alias("Gateway__Chain__")]
    type Address = [u8; 20];

    #[type_alias("Gateway__Chain__")]
    type Amount = u128;

    #[type_alias("Gateway__Chain__")]
    type CashIndex = u128;

    #[type_alias("Gateway__Chain__")]
    type Rate = u128;

    #[type_alias("Gateway__Chain__")]
    type Timestamp = u64;

    #[type_alias("Gateway__Chain__")]
    type Hash = [u8; 32];

    #[type_alias("Gateway__Chain__")]
    type PublicKey = [u8; 64];

    #[type_alias("Gateway__Chain__")]
    type Signature = [u8; 65];

    #[type_alias("Gateway__Chain__")]
    type EventId = gate::EventId;

    #[type_alias("Gateway__Chain__")]
    type Event = gate::Event;

    #[type_alias("Gateway__Chain__")]
    type Block = gate::Block;

    fn zero_hash() -> Self::Hash {
        panic!("XXX not implemented");
    }

    fn hash_bytes(_data: &[u8]) -> Self::Hash {
        panic!("XXX not implemented");
    }

    fn recover_user_address(
        _data: &[u8],
        _signature: Self::Signature,
    ) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn recover_address(_data: &[u8], _signature: Self::Signature) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn sign_message(_message: &[u8]) -> Result<Self::Signature, Reason> {
        panic!("XXX not implemented");
    }

    fn signer_address() -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn str_to_address(_addr: &str) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn address_string(_address: &Self::Address) -> String {
        panic!("XXX not implemented");
    }
}

impl Chain for Ethereum {
    const ID: ChainId = ChainId::Eth;

    #[type_alias("Ethereum__Chain__")]
    type Address = [u8; 20];

    #[type_alias("Ethereum__Chain__")]
    type Amount = u128;

    #[type_alias("Ethereum__Chain__")]
    type CashIndex = u128;

    #[type_alias("Ethereum__Chain__")]
    type Rate = u128;

    #[type_alias("Ethereum__Chain__")]
    type Timestamp = u64;

    #[type_alias("Ethereum__Chain__")]
    type Hash = [u8; 32];

    #[type_alias("Ethereum__Chain__")]
    type PublicKey = [u8; 64];

    #[type_alias("Ethereum__Chain__")]
    type Signature = [u8; 65];

    #[type_alias("Ethereum__Chain__")]
    type EventId = eth::EventId;

    #[type_alias("Ethereum__Chain__")]
    type Event = eth::Event;
    type Block = eth::Block;

    fn zero_hash() -> Self::Hash {
        [0u8; 32]
    }

    fn hash_bytes(data: &[u8]) -> Self::Hash {
        use tiny_keccak::Hasher;
        let mut hash = [0u8; 32];
        let mut hasher = tiny_keccak::Keccak::v256();
        hasher.update(&data[..]);
        hasher.finalize(&mut hash);
        hash
    }

    fn recover_user_address(
        data: &[u8],
        signature: Self::Signature,
    ) -> Result<Self::Address, Reason> {
        Ok(runtime_interfaces::keyring_interface::eth_recover(
            data.into(),
            signature,
            true,
        )?)
    }

    fn recover_address(data: &[u8], signature: Self::Signature) -> Result<Self::Address, Reason> {
        Ok(runtime_interfaces::keyring_interface::eth_recover(
            data.into(),
            signature,
            false,
        )?)
    }

    fn sign_message(message: &[u8]) -> Result<Self::Signature, Reason> {
        let message = Vec::from(message);
        let eth_key_id = runtime_interfaces::validator_config_interface::get_eth_key_id()
            .ok_or(Reason::KeyNotFound)?;
        Ok(runtime_interfaces::keyring_interface::sign_one(
            message, eth_key_id,
        )?)
    }

    fn signer_address() -> Result<Self::Address, Reason> {
        let eth_key_id = runtime_interfaces::validator_config_interface::get_eth_key_id()
            .ok_or(Reason::KeyNotFound)?;
        let pubk = runtime_interfaces::keyring_interface::get_public_key(eth_key_id)?;
        Ok(public_key_bytes_to_eth_address(&pubk))
    }

    fn str_to_address(addr: &str) -> Result<Self::Address, Reason> {
        match gateway_crypto::str_to_address(addr) {
            Some(s) => Ok(s),
            None => Err(Reason::BadAddress),
        }
    }

    fn address_string(address: &Self::Address) -> String {
        gateway_crypto::address_string(address)
    }
}

impl Chain for Polkadot {
    const ID: ChainId = ChainId::Dot;

    #[type_alias("Polkadot__Chain__")]
    type Address = [u8; 20];

    #[type_alias("Polkadot__Chain__")]
    type Amount = u128;

    #[type_alias("Polkadot__Chain__")]
    type CashIndex = u128;

    #[type_alias("Polkadot__Chain__")]
    type Rate = u128;

    #[type_alias("Polkadot__Chain__")]
    type Timestamp = u64;

    #[type_alias("Polkadot__Chain__")]
    type Hash = [u8; 32];

    #[type_alias("Polkadot__Chain__")]
    type PublicKey = [u8; 64];

    #[type_alias("Polkadot__Chain__")]
    type Signature = [u8; 65];

    #[type_alias("Polkadot__Chain__")]
    type EventId = dot::EventId;

    #[type_alias("Polkadot__Chain__")]
    type Event = dot::Event;
    type Block = dot::Block;

    fn zero_hash() -> Self::Hash {
        panic!("XXX not implemented");
    }

    fn hash_bytes(_data: &[u8]) -> Self::Hash {
        panic!("XXX not implemented");
    }

    fn recover_user_address(
        _data: &[u8],
        _signature: Self::Signature,
    ) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn recover_address(_data: &[u8], _signature: Self::Signature) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn sign_message(_message: &[u8]) -> Result<Self::Signature, Reason> {
        panic!("XXX not implemented");
    }

    fn signer_address() -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn str_to_address(_addr: &str) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn address_string(_address: &Self::Address) -> String {
        panic!("XXX not implemented");
    }
}

impl Chain for Solana {
    const ID: ChainId = ChainId::Sol;

    #[type_alias("Solana__Chain__")]
    type Address = [u8; 20];

    #[type_alias("Solana__Chain__")]
    type Amount = u128;

    #[type_alias("Solana__Chain__")]
    type CashIndex = u128;

    #[type_alias("Solana__Chain__")]
    type Rate = u128;

    #[type_alias("Solana__Chain__")]
    type Timestamp = u64;

    #[type_alias("Solana__Chain__")]
    type Hash = [u8; 32];

    #[type_alias("Solana__Chain__")]
    type PublicKey = [u8; 64];

    #[type_alias("Solana__Chain__")]
    type Signature = [u8; 65];

    #[type_alias("Solana__Chain__")]
    type EventId = sol::EventId;

    #[type_alias("Solana__Chain__")]
    type Event = sol::Event;
    type Block = sol::Block;

    fn zero_hash() -> Self::Hash {
        panic!("XXX not implemented");
    }

    fn hash_bytes(_data: &[u8]) -> Self::Hash {
        panic!("XXX not implemented");
    }

    fn recover_user_address(
        _data: &[u8],
        _signature: Self::Signature,
    ) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn recover_address(_data: &[u8], _signature: Self::Signature) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn sign_message(_message: &[u8]) -> Result<Self::Signature, Reason> {
        panic!("XXX not implemented");
    }

    fn signer_address() -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn str_to_address(_addr: &str) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn address_string(_address: &Self::Address) -> String {
        panic!("XXX not implemented");
    }
}

impl Chain for Tezos {
    const ID: ChainId = ChainId::Tez;

    #[type_alias("Tezos__Chain__")]
    type Address = [u8; 20];

    #[type_alias("Tezos__Chain__")]
    type Amount = u128;

    #[type_alias("Tezos__Chain__")]
    type CashIndex = u128;

    #[type_alias("Tezos__Chain__")]
    type Rate = u128;

    #[type_alias("Tezos__Chain__")]
    type Timestamp = u64;

    #[type_alias("Tezos__Chain__")]
    type Hash = [u8; 32];

    #[type_alias("Tezos__Chain__")]
    type PublicKey = [u8; 64];

    #[type_alias("Tezos__Chain__")]
    type Signature = [u8; 65];

    #[type_alias("Tezos__Chain__")]
    type EventId = tez::EventId;

    #[type_alias("Tezos__Chain__")]
    type Event = tez::Event;
    type Block = tez::Block;

    fn zero_hash() -> Self::Hash {
        panic!("XXX not implemented");
    }

    fn hash_bytes(_data: &[u8]) -> Self::Hash {
        panic!("XXX not implemented");
    }

    fn recover_user_address(
        _data: &[u8],
        _signature: Self::Signature,
    ) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn recover_address(_data: &[u8], _signature: Self::Signature) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn sign_message(_message: &[u8]) -> Result<Self::Signature, Reason> {
        panic!("XXX not implemented");
    }

    fn signer_address() -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn str_to_address(_addr: &str) -> Result<Self::Address, Reason> {
        panic!("XXX not implemented");
    }

    fn address_string(_address: &Self::Address) -> String {
        panic!("XXX not implemented");
    }
}

// XXX technically all the remaining mod::types I think could become ADTs instead
//  which would also be a union type that would allow us to store them together
//  in general storing types which add variants for chains over time *must* be ok
//   or this strategy breaks and we need to re-visit everywhere in storage that's happening
pub mod gate {
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    use types_derive::type_alias;

    #[type_alias("gate__")]
    pub type EventId = (u64, u64); // XXX

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {}

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Block {}
}

pub mod eth {
    // Note: The substrate build requires these be imported
    pub use our_std::vec::Vec;

    use super::{Chain, Ethereum};
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    use types_derive::type_alias;

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub enum RecoveryError {
        SignatureRecoveryError,
    }

    #[type_alias("eth__")]
    pub type BlockNumber = u64;

    #[type_alias("eth__")]
    pub type LogIndex = u64;

    #[type_alias("eth__")]
    pub type EventId = (BlockNumber, LogIndex);

    #[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {
        pub id: EventId,
        pub data: EventData,
    }

    #[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
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
            index: <Ethereum as Chain>::CashIndex,
        },

        Gov {
            extrinsics: Vec<Vec<u8>>,
        },
    }

    #[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Block {
        pub hash: <Ethereum as Chain>::Hash,
        pub parent_hash: <Ethereum as Chain>::Hash,
        pub number: BlockNumber,
        pub events: Vec<Event>,
    }
}

pub mod dot {
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    use types_derive::type_alias;

    #[type_alias("dot__")]
    pub type EventId = (u64, u64);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {}

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Block {}
}

pub mod sol {
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    use types_derive::type_alias;

    #[type_alias("sol__")]
    pub type EventId = (u64, u64);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {}

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Block {}
}

pub mod tez {
    use codec::{Decode, Encode};
    use our_std::RuntimeDebug;

    use types_derive::type_alias;

    #[type_alias("tez__")]
    pub type EventId = (u128, u128);

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Event {}

    #[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct Block {}
}
