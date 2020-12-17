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

pub trait Chain {
    type Address = [u8; 20];
    type Account = Self::Address;
    type Asset = Self::Address;
    type Hash = [u8; 32];
    type Public = [u8; 32];
}

#[derive(Debug)]
pub struct Ethereum {

}

// impl from<Vec<u8>> for <self::Ethereum as Chain>::Address {
//     fn from(x: Vec<u8>) -> Self {

//     } 
// }


#[derive(Debug)]
pub struct Polkadot {}

#[derive(Debug)]
pub struct Solana {}

#[derive(Debug)]
pub struct Tezos {}

impl Chain for Ethereum {

}
impl Chain for Polkadot {}
impl Chain for Solana {}
impl Chain for Tezos {}
