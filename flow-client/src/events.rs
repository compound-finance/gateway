use codec::{Decode, Encode};
use our_std::{Deserialize, Serialize, RuntimeDebug};

use types_derive::Types;

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types, Serialize, Deserialize)]
struct Lock {
    asset: String,
    // sender: String,
    recipient: String,
    amount: u128,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types, Serialize, Deserialize)]
pub enum FlowEvent {
    Lock {
        asset: String,
       // sender: String,
        recipient: String,
        amount: u128,
    }
    // NoticeInvoked {
    //     era_id: u32,
    //     era_index: u32,
    //     notice_hash: [u8; 32],
    //     result: Vec<u8>,
    // },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EventError {
    UnknownEventTopic(String),
    ErrorParsingData,
}

pub fn decode_event(topic: &str, data: &str) -> Result<FlowEvent, EventError> {
    match topic {
        "Lock" => {
            let event_res: serde_json::error::Result<Lock> = serde_json::from_str(&data);
            let event = event_res.map_err(|_| EventError::ErrorParsingData)?;

            Ok(FlowEvent::Lock {
                asset: event.asset,
                // sender: event.sender,
                recipient: event.recipient,
                amount: event.amount,
            })
        }
        _ => Err(EventError::UnknownEventTopic(topic.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_decode_lock_event() {
        let topic = "Lock";
        let data = "{\"asset\":\"FLOW\",\"recipient\":\"fc6346ab93540e97\",\"amount\":1000000000}";
        assert_eq!(
            decode_event(topic, data),
            Ok(FlowEvent::Lock {
                asset: String::from("FLOW"),
                recipient: String::from("fc6346ab93540e97"),
                amount: 1000000000
            })
        )
    }
}
