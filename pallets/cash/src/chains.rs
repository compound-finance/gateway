// XXX where should this live? with e.g. ethereum_client?
use codec::{Decode, Encode};
use sp_std::vec::Vec;

pub type Amount = u128; // XXX not really
pub type Index = u128; // XXX
pub type Rate = u128; // XXX
pub type Timestamp = u32; // XXX

pub type GenerationId = u32;
pub type WithinGenerationId = u32;
pub type NoticeId = (GenerationId, WithinGenerationId);

pub trait L1 {
    type Address = [u8; 20];
    type Account = Self::Address;
    type Asset = Self::Address;
    type Hash = [u8; 32];
    type Public = [u8; 32];
}

#[derive(Debug)]
pub struct Ethereum {}

#[derive(Debug)]
pub struct Polkadot {}

#[derive(Debug)]
pub struct Solana {}

#[derive(Debug)]
pub struct Tezos {}

impl L1 for Ethereum {}
impl L1 for Polkadot {}
impl L1 for Solana {}
impl L1 for Tezos {}

// Ethereum events types
pub type BlockNumber = u32;
pub type LogIndex = u32;
pub type EventId = (BlockNumber, LogIndex);
pub type EthPayload = Vec<u8>;

#[derive(Debug, Encode, Decode)]
pub enum EthereumEvent {
    LockEvent {
        id: EventId,
        //parent: Chain::Hash,
        // asset: Chain::Asset,
        // account: Chain::Account,
        // amount: Amount,
    },

    LockCashEvent {
        id: EventId,
        //parent: Chain::Hash,
        // account: Chain::Asset,
        // amount: Chain::Account,
        // cash_yield_index: Index,
    },
    GovEvent {
        id: EventId,
    },
}

#[derive(Debug)]
pub enum Notice<'a, Chain: L1> {
    ExtractionNotice {
        id: NoticeId,
        parent: Chain::Hash,
        asset: Chain::Asset,
        account: Chain::Account,
        amount: Amount,
    },

    CashExtractionNotice {
        id: NoticeId,
        parent: Chain::Hash,
        account: Chain::Asset,
        amount: Chain::Account,
        cash_yield_index: Index,
    },

    FutureYieldNotice {
        id: NoticeId,
        parent: Chain::Hash,
        next_cash_yield: Rate,
        next_cash_yield_start_at: Timestamp,
        next_cash_yield_index: Index,
    },

    SetSupplyCapNotice {
        id: NoticeId,
        parent: Chain::Hash,
        asset: Chain::Asset,
        amount: Amount,
    },

    ChangeAuthorityNotice {
        id: NoticeId,
        parent: Chain::Hash,
        new_authorities: &'a [Chain::Public],
    },
}
