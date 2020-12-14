// XXX where should this live? with e.g. ethereum_client?
pub mod eth {
    // Note: The substrate build requires these be imported
    pub use sp_std::vec::Vec;

    pub type Payload = Vec<u8>;
    pub type BlockNumber = u32;
    pub type LogIndex = u32;
    pub type EventId = (BlockNumber, LogIndex);

    #[derive(Clone, Copy)]
    pub struct Event {
        pub id: EventId,
    }

    pub fn decode(data: Vec<u8>) -> Event {
        Event { id: (13, 37) } // XXX
    }

    /// XXX Work on sending proper Payload,
    /// XXX is Decoding and encoding useless here
    pub fn encode(event: &Event) -> Vec<u8> {
        let (block_number, log_index): (u32, u32) = event.id;
        ethabi::encode(&[
            ethabi::token::Token::Int(block_number.into()),
            ethabi::token::Token::Int(log_index.into()),
        ])
    }
}

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
