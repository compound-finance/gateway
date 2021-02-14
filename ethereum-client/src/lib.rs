#[macro_use]
extern crate lazy_static;

pub mod events;

use crate::events::decode_event;
pub use crate::events::EthereumEvent;
use codec::{Decode, Encode};
use frame_support::debug;
use our_std::convert::TryInto;
use our_std::RuntimeDebug;
use serde::Deserialize;
use sp_runtime::offchain::{http, Duration};

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EthereumClientError {
    HttpIoError,
    HttpTimeout,
    HttpErrorCode(u16),
    InvalidUTF8,
    JsonParseError,
}

#[derive(Deserialize, Debug)]
pub struct ResponseError {
    pub message: Option<String>,
    pub code: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct EventsResponse<T> {
    pub id: Option<u64>,
    pub result: Option<Vec<T>>,
    pub error: Option<ResponseError>,
}

#[derive(Deserialize, Debug)]
pub struct BlockResponse {
    pub id: Option<u64>,
    pub result: Option<String>,
    pub error: Option<ResponseError>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogObject {
    /// true when the log was removed, due to a chain reorganization. false if it's a valid log.
    pub removed: Option<bool>,
    /// integer of the log index position in the block. null when its pending log.
    pub log_index: Option<String>,
    /// integer of the transactions index position log was created from. null when its pending log.
    pub transaction_index: Option<String>,
    /// 32 Bytes - hash of the transactions this log was created from. null when its pending log.
    pub transaction_hash: Option<String>,
    /// 32 Bytes - hash of the block where this log was in. null when its pending. null when its pending log.
    pub block_hash: Option<String>,
    /// the block number where this log was in. null when its pending. null when its pending log.
    pub block_number: Option<String>,
    /// 20 Bytes - address from which this log originated.
    pub address: Option<String>,
    /// contains one or more 32 Bytes non-indexed arguments of the log.
    pub data: Option<String>,
    /// Array of 0 to 4 32 Bytes of indexed log arguments. (In solidity: The first topic is the hash of the signature of the event (e.g. Deposit(address,bytes32,uint256)), except you declared the event with the anonymous specifier.)
    pub topics: Option<Vec<String>>,
}

fn deserialize_get_logs_response(
    response: &str,
) -> serde_json::error::Result<EventsResponse<LogObject>> {
    serde_json::from_str(response)
}

fn deserialize_get_block_number_response(
    response: &str,
) -> serde_json::error::Result<BlockResponse> {
    serde_json::from_str(response)
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct EthereumLogEvent {
    pub block_hash: [u8; 32],
    pub block_number: u64,
    pub transaction_index: u64,
    pub log_index: u64,
    pub event: EthereumEvent,
}

fn send_rpc(
    server: &str,
    method: &'static str,
    params: Vec<&str>,
) -> Result<String, EthereumClientError> {
    // TODO - move 2_000 to config???
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
    let data = format!(
        r#"{{"jsonrpc":"2.0","method":"{}","params":[{}],"id":1}}"#,
        method,
        params.join(",")
    );
    // debug::native::info!("Data for send_rpc: {}", data.clone());

    let request = http::Request::post(server, vec![data]);

    let pending = request
        .deadline(deadline)
        .add_header("Content-Type", "application/json")
        .send()
        .map_err(|_| EthereumClientError::HttpIoError)?;

    let response = pending
        .try_wait(deadline)
        .map_err(|_| EthereumClientError::HttpTimeout)?
        .map_err(|_| EthereumClientError::HttpTimeout)?;

    if response.code != 200 {
        debug::warn!("Unexpected status code: {}", response.code);
        return Err(EthereumClientError::HttpErrorCode(response.code));
    }

    let body = response.body().collect::<Vec<u8>>();

    // Create a str slice from the body.
    let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
        debug::warn!("No UTF8 body");
        EthereumClientError::InvalidUTF8
    })?;

    Ok(String::from(body_str))
}

fn parse_word(val_opt: Option<String>) -> Option<[u8; 32]> {
    match val_opt {
        Some(val) => match events::decode_hex(&val) {
            Ok(v) => match ethabi::decode(&[ethabi::ParamType::FixedBytes(32)], &v[..]) {
                Ok(tokens) => match &tokens[..] {
                    [ethabi::token::Token::FixedBytes(bytes)] => bytes[..].try_into().ok(),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        },
        None => None,
    }
}

// Note: our hex library won't even _parse_ hex with an odd-number of digits
//       so we need to pad before we parse with ethabi, as opposed to decoding
//       and then padding.
fn pad(val: String) -> Option<String> {
    if val.len() > 66 || &val[0..2] != "0x" {
        None
    } else {
        let mut s = String::with_capacity(64);
        let padding = 66 - val.len();
        for _ in 0..padding {
            s.push('0');
        }
        s.push_str(&val[2..]);
        Some(s)
    }
}

fn parse_u64(val_opt: Option<String>) -> Option<u64> {
    match val_opt {
        Some(val) => match pad(val) {
            Some(padded) => match hex::decode(&padded) {
                Ok(v) => match ethabi::decode(&[ethabi::ParamType::Uint(256)], &v[..]) {
                    Ok(tokens) => match tokens[..] {
                        [ethabi::token::Token::Uint(uint)] => uint.try_into().ok(),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            },
            None => None,
        },
        None => None,
    }
}

pub fn fetch_and_decode_logs(
    server: &str,
    params: Vec<&str>,
) -> Result<Vec<EthereumLogEvent>, EthereumClientError> {
    let body_str: String = send_rpc(server, "eth_getLogs", params)?;
    let deserialized_body = deserialize_get_logs_response(&body_str)
        .map_err(|_| EthereumClientError::JsonParseError)?;
    let eth_logs = deserialized_body
        .result
        .ok_or(EthereumClientError::JsonParseError)?;

    debug::native::info!("Eth Starport found {} log result(s)", eth_logs.len());

    Ok(eth_logs
        .into_iter()
        .filter_map(|eth_log| {
            println!("eth log {:?}", eth_log);
            match (
                parse_word(eth_log.block_hash),
                parse_u64(eth_log.transaction_index),
                eth_log.data,
                eth_log.topics,
                parse_u64(eth_log.block_number),
                parse_u64(eth_log.log_index),
            ) {
                (
                    Some(block_hash),
                    Some(transaction_index),
                    Some(data),
                    Some(topics),
                    Some(block_number),
                    Some(log_index),
                ) => match decode_event(topics, data) {
                    Ok(event) => Some(EthereumLogEvent {
                        block_hash,
                        block_number,
                        transaction_index,
                        log_index,
                        event,
                    }),
                    Err(err) => {
                        println!("Failed to parse log {:?}", err);
                        None
                    }
                },
                _ => {
                    println!("Missing critical field from eth log event");

                    None
                }
            }
        })
        .collect())
}

pub fn fetch_latest_block(server: &str) -> Result<u64, EthereumClientError> {
    let body_str: String = send_rpc(server, "eth_blockNumber", vec![])?;
    let deserialized_body = deserialize_get_block_number_response(&body_str)
        .map_err(|_| EthereumClientError::JsonParseError)?;

    parse_u64(Some(
        deserialized_body
            .result
            .ok_or(EthereumClientError::JsonParseError)?,
    ))
    .ok_or(EthereumClientError::JsonParseError)
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_deserialize_get_logs_request_happy_path() {
        const RESPONSE: &str = r#"{
      "jsonrpc": "2.0",
      "id": 1,
      "result": [
        {
          "address": "0x1a94fce7ef36bc90959e206ba569a12afbc91ca1",
          "blockHash": "0x7c5a35e9cb3e8ae0e221ab470abae9d446c3a5626ce6689fc777dcffcab52c70",
          "blockNumber": "0x5c29fb",
          "data": "0x0000000000000000000000003e3310720058c51f0de456e273c626cdd35065700000000000000000000000000000000000000000000000000000000000003185000000000000000000000000000000000000000000000000000000000000318200000000000000000000000000000000000000000000000000000000005c2a23",
          "logIndex": "0x1d",
          "removed": false,
          "topics": [
            "0x241ea03ca20251805084d27d4440371c34a0b85ff108f6bb5611248f73818b80"
          ],
          "transactionHash": "0x3dc91b98249fa9f2c5c37486a2427a3a7825be240c1c84961dfb3063d9c04d50",
          "transactionIndex": "0x1d"
        },
        {
          "address": "0x06012c8cf97bead5deae237070f9587f8e7a266d",
          "blockHash": "0x7c5a35e9cb3e8ae0e221ab470abae9d446c3a5626ce6689fc777dcffcab52c70",
          "blockNumber": "0x5c29fb",
          "data": "0x00000000000000000000000077ea137625739598666ded665953d26b3d8e374400000000000000000000000000000000000000000000000000000000000749ff00000000000000000000000000000000000000000000000000000000000a749d00000000000000000000000000000000000000000000000000000000005c2a0f",
          "logIndex": "0x57",
          "removed": false,
          "topics": [
            "0x241ea03ca20251805084d27d4440371c34a0b85ff108f6bb5611248f73818b80"
          ],
          "transactionHash": "0x788b1442414cb9c9a36dba2abe250763161a6f6395788a2e808f1b34e92beec1",
          "transactionIndex": "0x54"
        }
      ]
    }"#;
        let actual = deserialize_get_logs_response(RESPONSE);
        assert!(actual.is_ok());
        let actual = actual.unwrap();
        // println!("{:?}", actual);
        assert!(actual.id.is_some());
        assert_eq!(actual.id.unwrap(), 1);
        assert!(actual.result.is_some());
        assert!(actual.error.is_none());
        // todo : assert all the fields, but i inspected it, it is working fine.....
    }

    #[test]
    fn test_deserialize_get_logs_request_error_path() {
        const RESPONSE: &str = r#"{
      "jsonrpc": "2.0",
      "id": 1,
      "error": {
        "code": -32005,
        "message": "query returned more than 10000 results"
      }
    }"#;
        let actual = deserialize_get_logs_response(RESPONSE);
        assert!(actual.is_ok());
        let actual = actual.unwrap();
        // println!("{:?}", actual);
        assert!(actual.id.is_some());
        assert_eq!(actual.id.unwrap(), 1);
        assert!(actual.result.is_none());
        assert!(actual.error.is_some());
        // todo : assert all the fields, but i inspected it, it is working fine.....
    }

    #[test]
    fn test_deserialize_get_logs_request_totally_unexpected_input() {
        const RESPONSE: &str = r#"{"USD": 2}"#;
        let actual = deserialize_get_logs_response(RESPONSE);
        assert!(actual.is_ok());
        let actual = actual.unwrap();
        // println!("{:?}", actual);
        assert!(actual.id.is_none());
        assert!(actual.result.is_none());
        assert!(actual.error.is_none());
        // todo : assert all the fields, but i inspected it, it is working fine.....
    }

    // #[test]
    // fn test_decode_events() {
    //     // from https://kovan.etherscan.io/tx/0x1276fa72a2d8efec8e127dac6e57eb678e706cb4fbdd1b311bda75d2691b1941#eventlog
    //     const DATA_FIELD: &str = r#"0x000000000000000000000000d87ba7a50b2e7e660f678a895e4b72e7cb4ccd9c000000000000000000000000b819706e897eacf235cdb5048962bd65873202c400000000000000000000000000000000000000000000000000000000018cba80"#;
    //     let actual = decode_events(
    //         String::from(DATA_FIELD),
    //         vec![
    //             ethabi::param_type::ParamType::Address,
    //             ethabi::param_type::ParamType::Address,
    //             ethabi::param_type::ParamType::Uint(256),
    //         ],
    //     );
    //     if actual.is_err() {
    //         println!("{}", actual.err().unwrap());
    //         assert!(false);
    //     }
    // }
}
