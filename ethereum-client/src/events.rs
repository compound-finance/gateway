use crate::hex::{decode_hex, decode_topic};
use codec::{Decode, Encode};
use our_std::convert::TryInto;
use our_std::RuntimeDebug;

use types_derive::Types;

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum EthereumEvent {
    Lock {
        asset: [u8; 20],
        sender: [u8; 20],
        chain: String,
        recipient: [u8; 32],
        amount: u128,
    },
    LockCash {
        sender: [u8; 20],
        chain: String,
        recipient: [u8; 32],
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
                indexed: true
            },
            ethabi::EventParam {
                name: String::from("sender"),
                kind: ethabi::param_type::ParamType::Address,
                indexed: true
            },
            ethabi::EventParam {
                name: String::from("chain"),
                kind: ethabi::param_type::ParamType::String,
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("recipient"),
                kind: ethabi::param_type::ParamType::FixedBytes(32),
                indexed: true
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
                name: String::from("sender"),
                kind: ethabi::param_type::ParamType::Address,
                indexed: true
            },
            ethabi::EventParam {
                name: String::from("chain"),
                kind: ethabi::param_type::ParamType::String,
                indexed: false
            },
            ethabi::EventParam {
                name: String::from("recipient"),
                kind: ethabi::param_type::ParamType::FixedBytes(32),
                indexed: true
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
                indexed: true
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
                indexed: true
            },
            ethabi::EventParam {
                name: String::from("eraIndex"),
                kind: ethabi::param_type::ParamType::Uint(32),
                indexed: true
            },
            ethabi::EventParam {
                name: String::from("noticeHash"),
                kind: ethabi::param_type::ParamType::FixedBytes(32),
                indexed: true
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
        [ethabi::LogParam {
            value: ethabi::token::Token::Address(asset),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Address(sender),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::String(chain),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::FixedBytes(recipient),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Uint(amount),
            ..
        }] => Ok(EthereumEvent::Lock {
            asset: (*asset).into(),
            sender: (*sender).into(),
            chain: chain.into(),
            recipient: recipient[..]
                .try_into()
                .map_err(|_| EventError::InvalidRecipient)?,
            amount: (*amount).try_into().map_err(|_| EventError::Overflow)?,
        }),
        _ => Err(EventError::InvalidLogParams),
    }
}

fn parse_lock_cash_log(log: ethabi::Log) -> Result<EthereumEvent, EventError> {
    match &log.params[..] {
        [ethabi::LogParam {
            value: ethabi::token::Token::Address(sender),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::String(chain),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::FixedBytes(recipient),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Uint(amount),
            ..
        }, ethabi::LogParam {
            value: ethabi::token::Token::Uint(principal),
            ..
        }] => Ok(EthereumEvent::LockCash {
            sender: (*sender).into(),
            chain: chain.into(),
            recipient: recipient[..]
                .try_into()
                .map_err(|_| EventError::InvalidRecipient)?,
            amount: (*amount).try_into().map_err(|_| EventError::Overflow)?,
            principal: (*principal).try_into().map_err(|_| EventError::Overflow)?,
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
    UnknownEventTopic([u8; 32]),
    ErrorParsingLog,
    InvalidHex,
    InvalidTopic,
    Overflow,
    InvalidHash,
    InvalidLogParams,
    InvalidRecipient,
}

pub fn decode_event(topics: Vec<String>, data: String) -> Result<EthereumEvent, EventError> {
    let topic_hashes = topics
        .iter()
        .map(|topic| decode_topic(topic).ok_or(EventError::InvalidTopic))
        .collect::<Result<Vec<ethabi::Hash>, _>>()?;
    let topic_hash = topic_hashes.first().ok_or(EventError::InvalidTopic)?;
    if *topic_hash == *LOCK_EVENT_TOPIC {
        let log: ethabi::Log = LOCK_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data).ok_or(EventError::InvalidHex)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_lock_log(log)
    } else if *topic_hash == *LOCK_CASH_EVENT_TOPIC {
        let log: ethabi::Log = LOCK_CASH_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data).ok_or(EventError::InvalidHex)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_lock_cash_log(log)
    } else if *topic_hash == *EXEC_TRX_REQUEST_EVENT_TOPIC {
        let log: ethabi::Log = EXEC_TRX_REQUEST_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data).ok_or(EventError::InvalidHex)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_exec_trx_request_log(log)
    } else if *topic_hash == *EXECUTE_PROPOSAL_EVENT_TOPIC {
        let log: ethabi::Log = EXECUTE_PROPOSAL_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data).ok_or(EventError::InvalidHex)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_execute_proposal_log(log)
    } else if *topic_hash == *NOTICE_INVOKED_EVENT_TOPIC {
        let log: ethabi::Log = NOTICE_INVOKED_EVENT
            .parse_log(ethabi::RawLog {
                topics: topic_hashes,
                data: decode_hex(&data).ok_or(EventError::InvalidHex)?,
            })
            .map_err(|_| EventError::ErrorParsingLog)?;

        parse_notice_invoked_log(log)
    } else {
        Err(EventError::UnknownEventTopic(*topic_hash.as_fixed_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: these tests just come from copying and pasting `starport.js` unit test data.
    #[test]
    fn test_decode_lock_event() {
        let topics = vec![
            String::from("0xc459acef3ffe957663bb49d644b20d0c790bcb41573893752a72ba6f023b9386"),
            String::from("0x000000000000000000000000090c0328627d5cbd7e584c558694303d8ba6a239"),
            String::from("0x000000000000000000000000be974354c40d6e585804b0ee3552f18ec2eee1c9"),
            String::from("0xbe974354c40d6e585804b0ee3552f18ec2eee1c9000000000000000000000000"),
        ];

        let data =
            String::from("0x00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000");
        assert_eq!(
            decode_event(topics, data),
            Ok(EthereumEvent::Lock {
                asset: [
                    9, 12, 3, 40, 98, 125, 92, 189, 126, 88, 76, 85, 134, 148, 48, 61, 139, 166,
                    162, 57
                ],
                sender: [
                    190, 151, 67, 84, 196, 13, 110, 88, 88, 4, 176, 238, 53, 82, 241, 142, 194,
                    238, 225, 201
                ],
                chain: String::from("ETH"),
                recipient: [
                    190, 151, 67, 84, 196, 13, 110, 88, 88, 4, 176, 238, 53, 82, 241, 142, 194,
                    238, 225, 201, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                ],
                amount: 1000000000000000000
            })
        )
    }

    #[test]
    fn test_decode_lock_cash_event() {
        let topics = vec![
            String::from("0x0ba767ef2faa3001dbd3344d5b427be12f2e090ae3dcbe2f0d0ecf2bf17a8a17"),
            String::from("0x000000000000000000000000be974354c40d6e585804b0ee3552f18ec2eee1c9"),
            String::from("0xbe974354c40d6e585804b0ee3552f18ec2eee1c9000000000000000000000000"),
        ];

        let data =
            String::from("0x000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000");
        assert_eq!(
            decode_event(topics, data),
            Ok(EthereumEvent::LockCash {
                sender: [
                    190, 151, 67, 84, 196, 13, 110, 88, 88, 4, 176, 238, 53, 82, 241, 142, 194,
                    238, 225, 201
                ],
                chain: String::from("ETH"),
                recipient: [
                    190, 151, 67, 84, 196, 13, 110, 88, 88, 4, 176, 238, 53, 82, 241, 142, 194,
                    238, 225, 201, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                ],
                amount: 1000000,
                principal: 1000000,
            })
        )
    }

    #[test]
    fn test_decode_exec_trx_request_event() {
        let topics = vec![
            String::from("0xc25618d2506dbaa46f0a3819f68074c34ed888161951d0d833fea35b82a4faa9"),
            String::from("0x000000000000000000000000d8a1a591164cf36e9dfb9f9965924325b7e9fc9a"),
        ];
        let data =
            String::from("0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000412845787472616374203130302043415348204574683a3078643841314135393131363443463336453964464239463939363539323433323562374539466339612900000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(
            decode_event(topics, data),
            Ok(EthereumEvent::ExecTrxRequest {
                account: [
                    216, 161, 165, 145, 22, 76, 243, 110, 157, 251, 159, 153, 101, 146, 67, 37,
                    183, 233, 252, 154
                ],
                trx_request: String::from(
                    "(Extract 100 CASH Eth:0xd8A1A591164CF36E9dFB9F9965924325b7E9Fc9a)"
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
        let topics = vec![
            String::from("0xedd00d39b017eafbdd1eb7463087942ca834c96b1aa19e2a5ae97afef538c1a3"),
            String::from("0x0000000000000000000000000000000000000000000000000000000000000000"),
            String::from("0x0000000000000000000000000000000000000000000000000000000000000003"),
            String::from("0x1dcbdf2a45eb25eff04bf9f436341cecf99b05e5d1d2925991a7a2906c97a7b5"),
        ];
        let data =
            String::from("0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000001");
        assert_eq!(
            decode_event(topics, data),
            Ok(EthereumEvent::NoticeInvoked {
                era_id: 0,
                era_index: 3,
                notice_hash: [
                    29, 203, 223, 42, 69, 235, 37, 239, 240, 75, 249, 244, 54, 52, 28, 236, 249,
                    155, 5, 229, 209, 210, 146, 89, 145, 167, 162, 144, 108, 151, 167, 181
                ],
                result: vec![
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 1
                ],
            })
        )
    }
}
