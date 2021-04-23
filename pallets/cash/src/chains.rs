use crate::rates::APR;
use crate::reason::Reason;
use crate::types::{AssetAmount, CashIndex, SignersSet, Timestamp, ValidatorKeys};
use codec::{Decode, Encode};
use gateway_crypto::public_key_bytes_to_eth_address;
use our_std::vec::Vec;
use our_std::{
    collections::btree_set::BTreeSet, iter::Iterator, str::FromStr, vec, Debuggable, Deserialize,
    RuntimeDebug, Serialize,
};
use types_derive::{type_alias, Types};

/// Type for representing the selection of an underlying chain.
#[derive(Serialize, Deserialize)] // used in config
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainId {
    Eth,
    Dot,
}

impl ChainId {
    pub fn to_account(self, addr: &str) -> Result<ChainAccount, Reason> {
        match self {
            ChainId::Eth => Ok(ChainAccount::Eth(Ethereum::str_to_address(addr)?)),
            ChainId::Dot => Ok(ChainAccount::Dot(Polkadot::str_to_address(addr)?)),
        }
    }

    pub fn to_asset(self, addr: &str) -> Result<ChainAsset, Reason> {
        match self {
            ChainId::Eth => Ok(ChainAsset::Eth(Ethereum::str_to_address(addr)?)),
            ChainId::Dot => Ok(ChainAsset::Dot(Polkadot::str_to_address(addr)?)),
        }
    }

    pub fn to_hash(self, hash: &str) -> Result<ChainHash, Reason> {
        match self {
            ChainId::Eth => Ok(ChainHash::Eth(Ethereum::str_to_hash(hash)?)),
            ChainId::Dot => Ok(ChainHash::Dot(Polkadot::str_to_hash(hash)?)),
        }
    }

    pub fn signer_address(self) -> Result<ChainAccount, Reason> {
        match self {
            ChainId::Eth => Ok(ChainAccount::Eth(<Ethereum as Chain>::signer_address()?)),
            ChainId::Dot => Ok(ChainAccount::Dot(<Polkadot as Chain>::signer_address()?)),
        }
    }

    pub fn hash_bytes(self, data: &[u8]) -> ChainHash {
        match self {
            ChainId::Eth => ChainHash::Eth(<Ethereum as Chain>::hash_bytes(data)),
            ChainId::Dot => ChainHash::Dot(<Polkadot as Chain>::hash_bytes(data)),
        }
    }

    pub fn sign(self, message: &[u8]) -> Result<ChainSignature, Reason> {
        match self {
            ChainId::Eth => Ok(ChainSignature::Eth(<Ethereum as Chain>::sign_message(
                message,
            )?)),
            ChainId::Dot => Ok(ChainSignature::Dot(<Polkadot as Chain>::sign_message(
                message,
            )?)),
        }
    }

    pub fn starport_parent_block(self) -> ChainBlock {
        match self {
            ChainId::Eth => ChainBlock::Eth(
                runtime_interfaces::config_interface::get_eth_starport_parent_block(),
            ),
            _ => panic!("XXX not implemented"),
        }
    }

    pub fn zero_hash(self) -> ChainHash {
        match self {
            ChainId::Eth => ChainHash::Eth(<Ethereum as Chain>::zero_hash()),
            ChainId::Dot => ChainHash::Dot(<Polkadot as Chain>::zero_hash()),
        }
    }
}

/// Type for an account tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainAccount {
    Eth(<Ethereum as Chain>::Address),
    Dot(<Polkadot as Chain>::Address),
}

impl ChainAccount {
    pub fn chain_id(&self) -> ChainId {
        match *self {
            ChainAccount::Eth(_) => ChainId::Eth,
            ChainAccount::Dot(_) => ChainId::Dot,
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
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainAsset {
    Eth(<Ethereum as Chain>::Address),
    Dot(<Polkadot as Chain>::Address),
}

// For serialize (which we don't really use, but are required to implement)
impl ChainAsset {
    pub fn chain_id(&self) -> ChainId {
        match *self {
            ChainAsset::Eth(_) => ChainId::Eth,
            ChainAsset::Dot(_) => ChainId::Dot,
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
    Eth(<Ethereum as Chain>::Address, <Ethereum as Chain>::Signature),
    Dot(<Polkadot as Chain>::Address, <Polkadot as Chain>::Signature),
}

impl ChainAccountSignature {
    pub fn to_chain_signature(self) -> ChainSignature {
        match self {
            ChainAccountSignature::Eth(_, sig) => ChainSignature::Eth(sig),
            ChainAccountSignature::Dot(_, sig) => ChainSignature::Dot(sig),
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

/// Type for a hash tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainHash {
    Eth(<Ethereum as Chain>::Hash),
    Dot(<Polkadot as Chain>::Hash),
}

// Display so we can format local storage keys.
impl our_std::fmt::Display for ChainHash {
    fn fmt(&self, f: &mut our_std::fmt::Formatter<'_>) -> our_std::fmt::Result {
        match self {
            ChainHash::Eth(eth_hash) => write!(f, "ETH#{:X?}", eth_hash),
            ChainHash::Dot(dot_hash) => write!(f, "DOT#{:X?}", dot_hash),
        }
    }
}

// Implement deserialization for ChainHashes so we can use them in GenesisConfig / ChainSpec JSON.
//  i.e. "eth:0x..." <> Eth(0x...)
impl FromStr for ChainHash {
    type Err = Reason;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if let Some((chain_id_str, hash_str)) = String::from(string).split_once(":") {
            let chain_id = ChainId::from_str(chain_id_str)?;
            Ok(chain_id.to_hash(hash_str)?)
        } else {
            Err(Reason::BadHash)
        }
    }
}

impl From<ChainHash> for String {
    fn from(hash: ChainHash) -> String {
        match hash {
            ChainHash::Eth(eth_hash) => <Ethereum as Chain>::hash_string(&eth_hash),
            _ => panic!("XXX not implemented"),
        }
    }
}

/// Type for a signature tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainSignature {
    Eth(<Ethereum as Chain>::Signature),
    Dot(<Polkadot as Chain>::Signature),
}

impl ChainSignature {
    pub fn chain_id(&self) -> ChainId {
        match self {
            ChainSignature::Eth(_) => ChainId::Eth,
            ChainSignature::Dot(_) => ChainId::Dot,
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
    Eth(Vec<(<Ethereum as Chain>::Address, <Ethereum as Chain>::Signature)>),
    Dot(Vec<(<Polkadot as Chain>::Address, <Polkadot as Chain>::Signature)>),
}

impl ChainSignatureList {
    pub fn has_signer(&self, signer: ChainAccount) -> bool {
        match (self, signer) {
            (ChainSignatureList::Eth(eth_signature_pairs), ChainAccount::Eth(eth_account)) => {
                eth_signature_pairs.iter().any(|(s, _)| *s == eth_account)
            }
            _ => false,
        }
    }

    pub fn has_validator_signature(&self, chain_id: ChainId, validator: &ValidatorKeys) -> bool {
        match chain_id {
            ChainId::Eth => self.has_signer(ChainAccount::Eth(validator.eth_address)),
            _ => false,
        }
    }

    pub fn add_validator_signature(
        &mut self,
        signature: &ChainSignature,
        validator: &ValidatorKeys,
    ) -> Result<(), Reason> {
        match (self, signature) {
            (ChainSignatureList::Eth(eth_sig_list), ChainSignature::Eth(eth_sig)) => {
                Ok(eth_sig_list.push((validator.eth_address, eth_sig.clone())))
            }
            _ => Err(Reason::SignatureMismatch),
        }
    }
}

// Implement deserialization for ChainIds so we can use them in GenesisConfig / ChainSpec JSON.
impl FromStr for ChainId {
    type Err = Reason;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "ETH" => Ok(ChainId::Eth),
            "DOT" => Ok(ChainId::Dot),
            _ => Err(Reason::BadChainId),
        }
    }
}

/// Type for describing a block coming from an underlying chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
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
            ChainBlock::Eth(block) => block.number,
        }
    }

    pub fn events(&self) -> impl Iterator<Item = ChainBlockEvent> + '_ {
        match self {
            ChainBlock::Eth(block) => block
                .events
                .iter()
                .map(move |e| ChainBlockEvent::Eth(block.number, e.clone())),
        }
    }
}

/// Type for describing a set of blocks coming from an underlying chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainBlocks {
    Eth(Vec<<Ethereum as Chain>::Block>),
}

impl ChainBlocks {
    pub fn chain_id(&self) -> ChainId {
        match self {
            ChainBlocks::Eth(_) => ChainId::Eth,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            ChainBlocks::Eth(blocks) => blocks.len(),
        }
    }

    pub fn blocks(&self) -> impl Iterator<Item = ChainBlock> + '_ {
        match self {
            ChainBlocks::Eth(blocks) => blocks.iter().map(|b| ChainBlock::Eth(b.clone())),
        }
    }
}

impl From<ChainBlock> for ChainBlocks {
    fn from(block: ChainBlock) -> Self {
        match block {
            ChainBlock::Eth(block) => ChainBlocks::Eth(vec![block]),
        }
    }
}

/// Type for describing a reorg of an underlying chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
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
        match self {
            ChainReorg::Eth { .. } => ChainId::Eth,
        }
    }

    pub fn from_hash(&self) -> ChainHash {
        match self {
            ChainReorg::Eth { from_hash, .. } => ChainHash::Eth(*from_hash),
        }
    }

    pub fn reverse_blocks(&self) -> impl Iterator<Item = ChainBlock> + '_ {
        match self {
            ChainReorg::Eth { reverse_blocks, .. } => {
                reverse_blocks.iter().map(|b| ChainBlock::Eth(b.clone()))
            }
        }
    }

    pub fn forward_blocks(&self) -> impl Iterator<Item = ChainBlock> + '_ {
        match self {
            ChainReorg::Eth { forward_blocks, .. } => {
                forward_blocks.iter().map(|b| ChainBlock::Eth(b.clone()))
            }
        }
    }
}

/// Calculate whether the signers have a super majority of the given validator set.
pub fn has_super_majority<T: Ord>(signers: &BTreeSet<T>, validator_set: &BTreeSet<T>) -> bool {
    // using ⌈j/m⌉ = ⌊(j+m-1)/m⌋
    let valid_signers: Vec<_> = validator_set.intersection(&signers).collect();
    valid_signers.len() >= (2 * validator_set.len() + 3 - 1) / 3
}

/// Type for tallying signatures for an underlying chain block.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub struct ChainBlockTally {
    pub block: ChainBlock,
    pub support: SignersSet,
    pub dissent: SignersSet,
}

impl ChainBlockTally {
    pub fn new(chain_id: ChainId, block: ChainBlock, validator: &ValidatorKeys) -> ChainBlockTally {
        match chain_id {
            ChainId::Eth => ChainBlockTally {
                block,
                support: [validator.substrate_id.clone()].iter().cloned().collect(),
                dissent: SignersSet::new(),
            },

            _ => panic!("xxx not implemented"),
        }
    }

    pub fn add_support(&mut self, validator: &ValidatorKeys) {
        self.support.insert(validator.substrate_id.clone());
    }

    pub fn add_dissent(&mut self, validator: &ValidatorKeys) {
        self.dissent.insert(validator.substrate_id.clone());
    }

    pub fn has_enough_support(&self, validator_set: &SignersSet) -> bool {
        has_super_majority(&self.support, validator_set)
    }

    pub fn has_enough_dissent(&self, validator_set: &SignersSet) -> bool {
        has_super_majority(&self.dissent, validator_set)
    }
}

/// Type for tallying signatures for an underlying chain reorg.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub struct ChainReorgTally {
    pub reorg: ChainReorg,
    pub support: SignersSet,
}

impl ChainReorgTally {
    pub fn new(chain_id: ChainId, reorg: ChainReorg, validator: &ValidatorKeys) -> ChainReorgTally {
        match chain_id {
            ChainId::Eth => ChainReorgTally {
                reorg,
                support: [validator.substrate_id.clone()].iter().cloned().collect(),
            },

            _ => panic!("xxx not implemented"),
        }
    }

    pub fn add_support(&mut self, validator: &ValidatorKeys) {
        self.support.insert(validator.substrate_id.clone());
    }

    pub fn has_enough_support(&self, validator_set: &SignersSet) -> bool {
        has_super_majority(&self.support, validator_set)
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainBlockEvent {
    Eth(ChainBlockNumber, <Ethereum as Chain>::Event),
}

impl ChainBlockEvent {
    pub fn chain_id(&self) -> ChainId {
        match self {
            ChainBlockEvent::Eth(..) => ChainId::Eth,
        }
    }

    pub fn block_number(&self) -> ChainBlockNumber {
        match self {
            ChainBlockEvent::Eth(block_num, _) => *block_num,
        }
    }

    pub fn sign_event(&self) -> Result<ChainSignature, Reason> {
        self.chain_id().sign(&self.encode())
    }
}

/// Type for describing a set of events coming from an underlying chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChainBlockEvents {
    Eth(Vec<(ChainBlockNumber, <Ethereum as Chain>::Event)>),
}

impl ChainBlockEvents {
    /// Return an empty queue for the given chain.
    pub fn empty(chain_id: ChainId) -> ChainBlockEvents {
        match chain_id {
            ChainId::Eth => ChainBlockEvents::Eth(vec![]),
            _ => panic!("XXX not implemented"),
        }
    }

    /// Push the events from block onto this queue of events.
    pub fn push(&mut self, block: &ChainBlock) {
        match self {
            ChainBlockEvents::Eth(eth_block_events) => match block {
                ChainBlock::Eth(eth_block) => {
                    for event in eth_block.events.iter() {
                        eth_block_events.push((eth_block.number, event.clone()));
                    }
                }
            },
        }
    }

    /// Sift through these events, retaining only the ones which pass the given predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&ChainBlockEvent) -> bool,
    {
        match self {
            ChainBlockEvents::Eth(eth_block_events) => {
                eth_block_events.retain(|(b, e)| f(&ChainBlockEvent::Eth(*b, e.clone())));
            }
        }
    }

    /// Find the index of the given event on this queue, or none.
    pub fn position(&self, event: &ChainBlockEvent) -> Option<usize> {
        match self {
            ChainBlockEvents::Eth(eth_block_events) => match event {
                ChainBlockEvent::Eth(block_num, eth_block) => eth_block_events
                    .iter()
                    .position(|(b, e)| *b == *block_num && *e == *eth_block),
            },
        }
    }

    /// Remove the event at the given position.
    pub fn remove(&mut self, pos: usize) {
        match self {
            ChainBlockEvents::Eth(eth_block_events) => {
                eth_block_events.remove(pos);
            }
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
    fn str_to_hash(hash: &str) -> Result<Self::Hash, Reason>;
    fn hash_string(hash: &Self::Hash) -> String;
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Ethereum {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Polkadot {}

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
    type Event = ethereum_client::EthereumEvent;

    #[type_alias("Ethereum__Chain__")]
    type Block = ethereum_client::EthereumBlock;

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
        match gateway_crypto::eth_str_to_address(addr) {
            Some(s) => Ok(s),
            None => Err(Reason::BadAddress),
        }
    }

    fn address_string(address: &Self::Address) -> String {
        gateway_crypto::eth_address_string(address)
    }

    fn str_to_hash(hash: &str) -> Result<Self::Hash, Reason> {
        match gateway_crypto::eth_str_to_hash(hash) {
            Some(s) => Ok(s),
            None => Err(Reason::BadHash),
        }
    }

    fn hash_string(hash: &Self::Hash) -> String {
        gateway_crypto::eth_hash_string(hash)
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
    type Event = ();

    #[type_alias("Polkadot__Chain__")]
    type Block = ();

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

    fn str_to_hash(_hash: &str) -> Result<Self::Hash, Reason> {
        panic!("XXX not implemented");
    }

    fn hash_string(_hash: &Self::Hash) -> String {
        panic!("XXX not implemented");
    }
}

pub fn get_chain_account(chain: String, recipient: [u8; 32]) -> Result<ChainAccount, Reason> {
    match &chain.to_ascii_uppercase()[..] {
        "ETH" => {
            let mut eth_recipient: [u8; 20] = [0; 20];
            eth_recipient[..].clone_from_slice(&recipient[0..20]);

            Ok(ChainAccount::Eth(eth_recipient))
        }
        _ => Err(Reason::InvalidChain),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_client::{EthereumBlock, EthereumEvent};

    #[test]
    fn test_chain_events_push() {
        let mut a = ChainBlockEvents::Eth(vec![]);
        a.push(&ChainBlock::Eth(EthereumBlock {
            hash: [2u8; 32],
            parent_hash: [1u8; 32],
            number: 2,
            events: vec![],
        }));
        assert_eq!(a, ChainBlockEvents::Eth(vec![]));
        a.push(&ChainBlock::Eth(EthereumBlock {
            hash: [2u8; 32],
            parent_hash: [1u8; 32],
            number: 2,
            events: vec![EthereumEvent::Lock {
                asset: [4u8; 20],
                sender: [5u8; 20],
                chain: String::from("ETH"),
                recipient: [6u8; 32],
                amount: 100,
            }],
        }));
        assert_eq!(
            a,
            ChainBlockEvents::Eth(vec![(
                2,
                EthereumEvent::Lock {
                    asset: [4u8; 20],
                    sender: [5u8; 20],
                    chain: String::from("ETH"),
                    recipient: [6u8; 32],
                    amount: 100,
                }
            )])
        );
    }
}
