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
    ExecTrxRequest {
        account: [u8; 20],
        trx_request: String,
    },
    ExecuteProposal {
        title: String,
        extrinsics: Vec<Vec<u8>>,
    },
    NoticeInvoked {
        era_id: u32,
        era_index: u32,
        notice_hash: [u8; 32],
        result: Vec<u8>,
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
    static ref EXEC_TRX_REQUEST_EVENT: ethabi::Event = ethabi::Event {
        name: String::from("ExecTrxRequest"),
        inputs: vec![
            ethabi::EventParam {
                name: String::from("account"),
                kind: ethabi::param_type::ParamType::Address,
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("title"),
                kind: ethabi::param_type::ParamType::String,
                indexed: false
            },
        ],
        anonymous: false
    };
    static ref EXEC_TRX_REQUEST_EVENT_TOPIC: ethabi::Hash = EXEC_TRX_REQUEST_EVENT.signature();
    static ref EXECUTE_PROPOSAL_EVENT: ethabi::Event = ethabi::Event {
        name: String::from("ExecuteProposal"),
        inputs: vec![
            ethabi::EventParam {
                name: String::from("title"),
                kind: ethabi::param_type::ParamType::String,
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("extrinsics"),
                kind: ethabi::param_type::ParamType::Array(Box::new(
                    ethabi::param_type::ParamType::Bytes
                )),
                indexed: false
            }
        ],
        anonymous: false
    };
    static ref EXECUTE_PROPOSAL_EVENT_TOPIC: ethabi::Hash = EXECUTE_PROPOSAL_EVENT.signature();
    static ref NOTICE_INVOKED_EVENT: ethabi::Event = ethabi::Event {
        name: String::from("NoticeInvoked"),
        inputs: vec![
            ethabi::EventParam {
                name: String::from("eraId"),
                kind: ethabi::param_type::ParamType::Uint(32),
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("eraIndex"),
                kind: ethabi::param_type::ParamType::Uint(32),
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("noticeHash"),
                kind: ethabi::param_type::ParamType::FixedBytes(32),
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("result"),
                kind: ethabi::param_type::ParamType::Bytes,
                indexed: false
            },
        ],
        anonymous: false
    };
    static ref NOTICE_INVOKED_EVENT_TOPIC: ethabi::Hash = NOTICE_INVOKED_EVENT.signature();
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

fn parse_exec_trx_request_log(log: ethabi::Log) -> Result<EthereumEvent, EventError> {
    match &log.params[..] {
        [ethabi::LogParam {
            value: ethabi::token::Token::Address(account),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::String(trx_request),
            ..
        }] => Ok(EthereumEvent::ExecTrxRequest {
            account: (*account).into(),
            trx_request: trx_request.into(),
        }),
        _ => Err(EventError::InvalidLogParams),
    }
}

fn parse_execute_proposal_log(log: ethabi::Log) -> Result<EthereumEvent, EventError> {
    match &log.params[..] {
        [ethabi::LogParam {
            value: ethabi::token::Token::String(title),
            ..
        }, ethabi::LogParam {
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
            Ok(EthereumEvent::ExecuteProposal {
                title: title.into(),
                extrinsics,
            })
        }
        _ => Err(EventError::InvalidLogParams),
    }
}

fn parse_notice_invoked_log(log: ethabi::Log) -> Result<EthereumEvent, EventError> {
    match &log.params[..] {
        [ethabi::LogParam {
            value: ethabi::token::Token::Uint(era_id),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Uint(era_index),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::FixedBytes(notice_hash),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Bytes(result),
            ..
        }] => Ok(EthereumEvent::NoticeInvoked {
            era_id: (*era_id).try_into().map_err(|_| EventError::Overflow)?,
            era_index: (*era_index).try_into().map_err(|_| EventError::Overflow)?,
            notice_hash: notice_hash[..]
                .try_into()
                .map_err(|_| EventError::InvalidHash)?,
            result: result.clone(), // TODO: Why the clones?
        }),
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
    InvalidHash,
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
    } else if *topic_hash == *EXEC_TRX_REQUEST_EVENT_TOPIC {
        let log: ethabi::Log = EXEC_TRX_REQUEST_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_exec_trx_request_log(log)
    } else if *topic_hash == *EXECUTE_PROPOSAL_EVENT_TOPIC {
        let log: ethabi::Log = EXECUTE_PROPOSAL_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_execute_proposal_log(log)
    } else if *topic_hash == *NOTICE_INVOKED_EVENT_TOPIC {
        let log: ethabi::Log = NOTICE_INVOKED_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_notice_invoked_log(log)
    } else {
        Err(EventError::UnknownEventTopic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: these tests just come from copying and pasting `starport.js` unit test data.
    #[test]
    fn test_decode_exec_trx_request_event() {
        let topics = vec![String::from(
            "0xc25618d2506dbaa46f0a3819f68074c34ed888161951d0d833fea35b82a4faa9",
        )];
        let data =
            String::from("0x00000000000000000000000028056190d4a5905caf3647c5987c4f26e0d9d935000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000412845787472616374203130302043415348204574683a3078323830353631393044344135393035636166333634374335393837433466323645304439443933352900000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(
            decode_event(topics, data),
            Ok(EthereumEvent::ExecTrxRequest {
                account: [
                    40, 5, 97, 144, 212, 165, 144, 92, 175, 54, 71, 197, 152, 124, 79, 38, 224,
                    217, 217, 53
                ],
                trx_request: String::from(
                    "(Extract 100 CASH Eth:0x28056190D4A5905caf3647C5987C4f26E0D9D935)"
                ),
            })
        )
    }

    #[test]
    fn test_decode_execute_proposal_event() {
        let topics = vec![String::from(
            "0x97b9e105962881d0aea472b7f0335a84c21cce09bc7917f3db0ea5e4b23116e8",
        )];
        let data =
            String::from("0x0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000094d7920416374696f6e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000003010203000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000030405060000000000000000000000000000000000000000000000000000000000");
        assert_eq!(
            decode_event(topics, data),
            Ok(EthereumEvent::ExecuteProposal {
                title: String::from("My Action"),
                extrinsics: vec![vec![1, 2, 3], vec![4, 5, 6]]
            })
        )
    }

    #[test]
    fn test_decode_notice_invoked_event() {
        let topics = vec![String::from(
            "0xedd00d39b017eafbdd1eb7463087942ca834c96b1aa19e2a5ae97afef538c1a3",
        )];
        let data =
            String::from("0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000349cdbb0243b4c762c76bb3e56d84c8d6b0d6d256f6a47088902018bf89048979000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000001");
        assert_eq!(
            decode_event(topics, data),
            Ok(EthereumEvent::NoticeInvoked {
                era_id: 0,
                era_index: 3,
                notice_hash: [
                    73, 205, 187, 2, 67, 180, 199, 98, 199, 107, 179, 229, 109, 132, 200, 214, 176,
                    214, 210, 86, 246, 164, 112, 136, 144, 32, 24, 191, 137, 4, 137, 121
                ],
                result: vec![
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 1
                ],
            })
        )
    }
}
