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

    /// XXX
    pub fn encode(event: &Event) -> Vec<u8> {
        let (block_number, log_index): (u32, u32) = event.id;
        ethabi::encode(&[
            ethabi::token::Token::Int(block_number.clone().into()),
            ethabi::token::Token::Int(log_index.clone().into()),
        ])
    }
}
