#[macro_use]
extern crate lazy_static;

pub mod events;
pub mod hex;

pub use crate::events::EthereumEvent;
use crate::{
    events::decode_event,
    hex::{parse_u64, parse_word},
};

use codec::{Decode, Encode};
use frame_support::debug;
use serde::Deserialize;
use sp_runtime::offchain::{http, Duration};

use our_std::RuntimeDebug;
use types_derive::Types;

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EthereumClientError {
    HttpIoError,
    HttpTimeout,
    HttpErrorCode(u16),
    InvalidUTF8,
    JsonParseError,
}

#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct ResponseError {
    pub message: Option<String>,
    pub code: Option<i64>,
}

#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct EventsResponse<T> {
    pub id: Option<u64>,
    pub result: Option<Vec<T>>,
    pub error: Option<ResponseError>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct TransactionObject {
    pub blockHash: Option<String>,
    pub blockNumber: Option<String>,
    pub from: Option<String>,
    pub gas: Option<String>,
    pub gasPrice: Option<String>,
    pub hash: Option<String>,
    pub input: Option<String>,
    pub nonce: Option<String>,
    pub to: Option<String>,
    pub transactionIndex: Option<String>,
    pub r#type: Option<String>,
    pub value: Option<String>,
    pub r: Option<String>,
    pub s: Option<String>,
    pub v: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct BlockObject {
    pub difficulty: Option<String>,
    pub extraData: Option<String>,
    pub gasLimit: Option<String>,
    pub gasUsed: Option<String>,
    pub hash: Option<String>,
    pub logsBloom: Option<String>,
    pub miner: Option<String>,
    pub mixHash: Option<String>,
    pub nonce: Option<String>,
    pub number: Option<String>,
    pub parentHash: Option<String>,
    pub receiptRoot: Option<String>,
    pub sha3Uncles: Option<String>,
    pub size: Option<String>,
    pub stateRoot: Option<String>,
    pub timestamp: Option<String>,
    pub totalDifficulty: Option<String>,
    pub transactions: Option<Vec<TransactionObject>>,
    pub transactionsRoot: Option<String>,
    pub uncles: Option<Vec<String>>,
}

#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct BlockResponse {
    pub id: Option<u64>,
    pub result: Option<BlockObject>,
    pub error: Option<ResponseError>,
}

#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct BlockNumberResponse {
    pub id: Option<u64>,
    pub result: Option<String>,
    pub error: Option<ResponseError>,
}

#[derive(Deserialize, RuntimeDebug, PartialEq)]
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

fn deserialize_get_block_by_number_response(
    response: &str,
) -> serde_json::error::Result<BlockResponse> {
    serde_json::from_str(response)
}

fn deserialize_block_number_response(
    response: &str,
) -> serde_json::error::Result<BlockNumberResponse> {
    serde_json::from_str(response)
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub struct EthereumLogEvent {
    pub block_hash: [u8; 32],
    pub block_number: u64,
    pub transaction_index: u64,
    pub log_index: u64,
    pub event: EthereumEvent,
}

fn send_rpc(
    server: &str,
    method: serde_json::Value,
    params: Vec<serde_json::Value>,
) -> Result<String, EthereumClientError> {
    // TODO - move 2_000 to config???
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
    let data = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id":1
    })
    .to_string();

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

pub fn fetch_and_decode_logs(
    server: &str,
    params: Vec<serde_json::Value>,
) -> Result<Vec<EthereumLogEvent>, EthereumClientError> {
    let body_str: String = send_rpc(server, "eth_getLogs".into(), params)?;
    let deserialized_body = deserialize_get_logs_response(&body_str)
        .map_err(|_| EthereumClientError::JsonParseError)?;
    let eth_logs = deserialized_body
        .result
        .ok_or(EthereumClientError::JsonParseError)?;

    if eth_logs.len() > 0 {
        debug::native::info!("Eth Starport found {} logs", eth_logs.len());
    }

    Ok(eth_logs
        .into_iter()
        .filter_map(|eth_log| {
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

pub fn fetch_block_with_number(
    server: &str,
    block_num: &str,
) -> Result<BlockObject, EthereumClientError> {
    let body_str: String = send_rpc(
        server,
        "eth_getBlockByNumber".into(),
        vec![block_num.into(), true.into()],
    )?;
    let deserialized_body = deserialize_get_block_by_number_response(&body_str)
        .map_err(|_| EthereumClientError::JsonParseError)?;

    deserialized_body
        .result
        .ok_or(EthereumClientError::JsonParseError)
}

pub fn fetch_latest_block(server: &str) -> Result<u64, EthereumClientError> {
    let body_str: String = send_rpc(server, "eth_blockNumber".into(), vec![])?;
    let deserialized_body = deserialize_block_number_response(&body_str)
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

    use sp_core::offchain::{testing, OffchainExt};

    #[test]
    fn test_fetch_latest_block() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = sp_io::TestExternalities::default();
        t.register_extension(OffchainExt::new(offchain));
        {
            let mut s = state.write();
            s.expect_request(testing::PendingRequest {
                method: "POST".into(),
                uri: "https://mainnet-eth.compound.finance".into(),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                body: br#"{"id":1,"jsonrpc":"2.0","method":"eth_blockNumber","params":[]}"#
                    .to_vec(),
                response: Some(br#"{"jsonrpc":"2.0","id":1,"result": "0x123"}"#.to_vec()),
                sent: true,
                ..Default::default()
            });
        }
        t.execute_with(|| {
            let result = fetch_latest_block("https://mainnet-eth.compound.finance");
            assert_eq!(result, Ok(291));
        });
    }

    #[test]
    fn test_fetch_block_with_number() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = sp_io::TestExternalities::default();
        t.register_extension(OffchainExt::new(offchain));
        {
            let mut s = state.write();
            s.expect_request(
                testing::PendingRequest {
                    method: "POST".into(),
                    uri: "https://mainnet-eth.compound.finance".into(),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: br#"{"id":1,"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x506",true]}"#.to_vec(),
                    response: Some(br#"{"jsonrpc":"2.0","id":1,"result":{"difficulty":"0xb9e274f7969f5","extraData":"0x65746865726d696e652d657531","gasLimit":"0x7a121d","gasUsed":"0x781503","hash":"0x61314c1c6837e15e60c5b6732f092118dd25e3ec681f5e089b3a9ad2374e5a8a","logsBloom":"0x044410ea904e1020440110008000902200168801c81010301489212010002008080b0010004001b006040222c42004b001200408400500901889c908212040401020008d300010100198d10800100080027900254120000000530141030808140c299400162c0000d200204080008838240009002c020010400010101000481660200420a884b8020282204a00141ce10805004810800190180114180001b0001b1000020ac8040007000320b0480004018240891882a20080010281002c00000010102e0184210003010100438004202003080401000806204010000a42200104110100201200008081005001104002410140114a002010808c00200894c0c0","miner":"0xea674fdde714fd979de3edf0f56aa9716b898ec8","mixHash":"0xd733e12126a2155f0278c3987777eaca558a274b42d0396306dffb8fa6d21e76","nonce":"0x56a66f3802150748","number":"0x506","parentHash":"0x062e77dced431eb671a56839f96da912f68d841024665748d38cd3d6795961ea","receiptsRoot":"0x19ad317358916207491d4b64340153b924f4dda88fa8ef5dcb49090f234c00e7","sha3Uncles":"0xd21bed33f01dac18a3ee5538d1607ff2709d742eb4e13877cf66dcbed6c980f2","size":"0x5f50","stateRoot":"0x40b48fa241b8f9749af10a5dd1dfb8db245ba94cbb4969ab5c5b905a6adfe5f6","timestamp":"0x5aae89b9","totalDifficulty":"0xa91291ae5c752d4885","transactions":[{"blockHash":"0x61314c1c6837e15e60c5b6732f092118dd25e3ec681f5e089b3a9ad2374e5a8a","blockNumber":"0x508990","from":"0x22b84d5ffea8b801c0422afe752377a64aa738c2","gas":"0x186a0","gasPrice":"0x153005ce00","hash":"0x94859e5d00b6bc572f877eaae906c0093eb22267d2d84d720ac90627fc63147c","input":"0x","nonce":"0x6740d","r":"0x5fc50bea42bc3d8c5f47790b92fbd79fa296f90fea4d35f1621001f6316a1b91","s":"0x774a47ca2112dd815f3bda90d537dfcdab0082f6bfca7262f91df258addf5706","to":"0x1d53de4d66110689bf494a110e859f3a6d15661f","transactionIndex":"0x0","type":"0x0","v":"0x25","value":"0x453aa4214124000"}],"transactionsRoot":"0xa46bb7bc06d4ad700df4100095fecd5a5af2994b6d1d24162ded673b7d485610","uncles":["0x5e7dde2e3811b5881a062c8b2ff7fd14687d79745e2384965d73a9df3fb0b4a8"]}}"#.to_vec()),
                    sent: true,
                    ..Default::default()
                });
        }
        t.execute_with(|| {
            let result = fetch_block_with_number("https://mainnet-eth.compound.finance", "0x506");
            let block = result.unwrap();
            assert_eq!(block.difficulty, Some("0xb9e274f7969f5".into()));
            assert_eq!(block.number, Some("0x506".into()));
            assert_eq!(block.transactions.unwrap().len(), 1);
            assert_eq!(block.uncles.unwrap().len(), 1);
        });
    }

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
        let result = deserialize_get_logs_response(RESPONSE);

        let expected = EventsResponse {
            id: Some(1),
            result: Some(vec![
                LogObject {
                    removed: Some(false),
                    log_index: Some(String::from("0x1d")),
                    transaction_index: Some(String::from("0x1d")),
                    transaction_hash: Some(String::from("0x3dc91b98249fa9f2c5c37486a2427a3a7825be240c1c84961dfb3063d9c04d50")),
                    block_hash: Some(String::from("0x7c5a35e9cb3e8ae0e221ab470abae9d446c3a5626ce6689fc777dcffcab52c70")),
                    block_number: Some(String::from("0x5c29fb")),
                    address: Some(String::from("0x1a94fce7ef36bc90959e206ba569a12afbc91ca1")),
                    data: Some(String::from("0x0000000000000000000000003e3310720058c51f0de456e273c626cdd35065700000000000000000000000000000000000000000000000000000000000003185000000000000000000000000000000000000000000000000000000000000318200000000000000000000000000000000000000000000000000000000005c2a23")),
                    topics: Some(vec![String::from("0x241ea03ca20251805084d27d4440371c34a0b85ff108f6bb5611248f73818b80")])
                },
                LogObject {
                    removed: Some(false),
                    log_index: Some(String::from("0x57")),
                    transaction_index: Some(String::from("0x54")),
                    transaction_hash: Some(String::from("0x788b1442414cb9c9a36dba2abe250763161a6f6395788a2e808f1b34e92beec1")),
                    block_hash: Some(String::from("0x7c5a35e9cb3e8ae0e221ab470abae9d446c3a5626ce6689fc777dcffcab52c70")),
                    block_number: Some(String::from("0x5c29fb")),
                    address: Some(String::from("0x06012c8cf97bead5deae237070f9587f8e7a266d")),
                    data: Some(String::from("0x00000000000000000000000077ea137625739598666ded665953d26b3d8e374400000000000000000000000000000000000000000000000000000000000749ff00000000000000000000000000000000000000000000000000000000000a749d00000000000000000000000000000000000000000000000000000000005c2a0f")),
                    topics: Some(vec![String::from("0x241ea03ca20251805084d27d4440371c34a0b85ff108f6bb5611248f73818b80")])
                }
            ]),
            error: None,
        };
        assert_eq!(result.unwrap(), expected)
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
        let result = deserialize_get_logs_response(RESPONSE);
        let expected = EventsResponse {
            id: Some(1),
            result: None,
            error: Some(ResponseError {
                message: Some(String::from("query returned more than 10000 results")),
                code: Some(-32005),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_deserialize_get_logs_request_totally_unexpected_input() {
        const RESPONSE: &str = r#"{"USD": 2}"#;
        let result = deserialize_get_logs_response(RESPONSE);
        let expected = EventsResponse {
            id: None,
            result: None,
            error: None,
        };
        assert_eq!(result.unwrap(), expected);
    }
}
