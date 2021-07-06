use codec::{Decode, Encode};
use our_std::convert::TryInto;
use our_std::{Deserialize, RuntimeDebug, Serialize};

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
        asset: [u8; 8],
        // sender: String,
        recipient: [u8; 8],
        amount: u128,
    }, // NoticeInvoked {
       //     era_id: u32,
       //     era_index: u32,
       //     notice_hash: [u8; 32],
       //     result: Vec<u8>,
       // }
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

            let mut asset_res: [u8; 8] = [0; 8];
            for (i, elem) in event.asset.as_bytes().iter().enumerate() {
                asset_res[i] = *elem;
                if i == 7 {
                    break;
                }
            }

            Ok(FlowEvent::Lock {
                asset: asset_res,
                // sender: event.sender,
                recipient: hex::decode(event.recipient)
                    .map_err(|_| EventError::ErrorParsingData)?
                    .try_into()
                    .map_err(|_| EventError::ErrorParsingData)?,
                amount: event.amount,
            })
        }
        _ => Err(EventError::UnknownEventTopic(topic.to_string())),
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
                asset: [70, 76, 79, 87, 0, 0, 0, 0], // "FLOW" asset
                recipient: hex::decode("fc6346ab93540e97").unwrap().try_into().unwrap(),
                amount: 1000000000
            })
        )
    }
}
