use codec::{Decode, Encode};
use our_std::convert::TryInto;
use our_std::RuntimeDebug;

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EthereumEvent {
    Lock {
        asset: [u8; 20],
        holder: [u8; 20],
        amount: u128,
    },
    LockCash {
        holder: [u8; 20],
        amount: u128,
        principal: u128,
    },
    Gov {
        extrinsics: Vec<Vec<u8>>,
    },
}

lazy_static! {
    static ref LOCK_EVENT: ethabi::Event = ethabi::Event {
        name: String::from("Lock"),
        inputs: vec![
            ethabi::EventParam {
                name: String::from("asset"),
                kind: ethabi::param_type::ParamType::Address,
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("holder"),
                kind: ethabi::param_type::ParamType::Address,
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("amount"),
                kind: ethabi::param_type::ParamType::Uint(256),
                indexed: false
            }
        ],
        anonymous: false
    };
    static ref LOCK_EVENT_TOPIC: ethabi::Hash = LOCK_EVENT.signature();
    static ref LOCK_CASH_EVENT: ethabi::Event = ethabi::Event {
        name: String::from("LockCash"),
        inputs: vec![
            ethabi::EventParam {
                name: String::from("holder"),
                kind: ethabi::param_type::ParamType::Address,
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("amount"),
                kind: ethabi::param_type::ParamType::Uint(256),
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("principal"),
                kind: ethabi::param_type::ParamType::Uint(128),
                indexed: false
            },
        ],
        anonymous: false
    };
    static ref LOCK_CASH_EVENT_TOPIC: ethabi::Hash = LOCK_CASH_EVENT.signature();
    static ref GOV_EVENT: ethabi::Event = ethabi::Event {
        name: String::from("Gov"),
        inputs: vec![ethabi::EventParam {
            name: String::from("extrinsics"),
            kind: ethabi::param_type::ParamType::Array(Box::new(
                ethabi::param_type::ParamType::Bytes
            )),
            indexed: false
        }],
        anonymous: false
    };
    static ref GOV_EVENT_TOPIC: ethabi::Hash = GOV_EVENT.signature();
}

fn parse_lock_log(log: ethabi::Log) -> Result<EthereumEvent, EventError> {
    match &log.params[..] {
        &[ethabi::LogParam {
            value: ethabi::token::Token::Address(asset),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Address(holder),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Uint(amount),
            ..
        }] => Ok(EthereumEvent::Lock {
            asset: asset.into(),
            holder: holder.into(),
            amount: amount.try_into().map_err(|_| EventError::Overflow)?,
        }),
        _ => Err(EventError::InvalidLogParams),
    }
}

fn parse_lock_cash_log(log: ethabi::Log) -> Result<EthereumEvent, EventError> {
    match &log.params[..] {
        &[ethabi::LogParam {
            value: ethabi::token::Token::Address(holder),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Uint(amount),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Uint(principal),
            ..
        }] => Ok(EthereumEvent::LockCash {
            holder: holder.into(),
            amount: amount.try_into().map_err(|_| EventError::Overflow)?,
            principal: principal.try_into().map_err(|_| EventError::Overflow)?,
        }),
        _ => Err(EventError::InvalidLogParams),
    }
}

fn parse_gov_log(log: ethabi::Log) -> Result<EthereumEvent, EventError> {
    match &log.params[..] {
        [ethabi::LogParam {
            value: ethabi::token::Token::Array(extrinsics_tokens),
            ..
        }] => {
            let extrinsics = extrinsics_tokens
                .clone()
                .into_iter()
                .map(|extrinsic| match extrinsic {
                    ethabi::token::Token::Bytes(extrinsic) => Ok(extrinsic),
                    _ => Err(EventError::InvalidLogParams),
                })
                .collect::<Result<Vec<Vec<u8>>, _>>()?;
            Ok(EthereumEvent::Gov { extrinsics })
        }
        _ => Err(EventError::InvalidLogParams),
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EventError {
    UnknownEventTopic,
    ErrorParsingLog,
    InvalidHex,
    InvalidTopic,
    Overflow,
    InvalidLogParams,
}

pub fn decode_hex(data: &String) -> Result<Vec<u8>, EventError> {
    if data.len() < 2 || &data[0..2] != "0x" {
        return Err(EventError::InvalidHex);
    }

    hex::decode(&data[2..]).map_err(|_| EventError::InvalidHex)
}

pub fn decode_topic(topic: &String) -> Result<ethabi::Hash, EventError> {
    let res = decode_hex(topic)?;
    let addr: &[u8; 32] = &res[..].try_into().map_err(|_| EventError::InvalidTopic)?;
    Ok(addr.into())
}

pub fn decode_event(topics: Vec<String>, data: String) -> Result<EthereumEvent, EventError> {
    let topic_hashes = topics
        .iter()
        .map(|topic| decode_topic(topic))
        .collect::<Result<Vec<ethabi::Hash>, _>>()?;
    let topic_hash = topic_hashes.first().ok_or(EventError::InvalidTopic)?;
    if *topic_hash == *LOCK_EVENT_TOPIC {
        let log: ethabi::Log = LOCK_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_lock_log(log)
    } else if *topic_hash == *LOCK_CASH_EVENT_TOPIC {
        let log: ethabi::Log = LOCK_CASH_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_lock_cash_log(log)
    } else if *topic_hash == *GOV_EVENT_TOPIC {
        let log: ethabi::Log = GOV_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_gov_log(log)
    } else {
        Err(EventError::UnknownEventTopic)
    }
}
