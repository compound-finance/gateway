#[macro_use]
extern crate lazy_static;

use codec::{Decode, Encode};
use hex_buffer_serde::{ConstHex, ConstHexForm};
use sp_runtime::offchain::{http, Duration};
use sp_runtime_interface::pass_by::PassByCodec;

use our_std::{debug, error, info, trace, warn, Deserialize, RuntimeDebug, Serialize};
use types_derive::{type_alias, Types};

pub mod events;
pub mod hex;

pub use crate::events::EthereumEvent;
pub use crate::hex::{parse_u64, parse_word};

#[type_alias]
pub type EthereumBlockNumber = u64;

#[type_alias]
pub type EthereumHash = [u8; 32];

const ETH_FETCH_DEADLINE: u64 = 10_000;

#[derive(Clone, RuntimeDebug)]
pub enum EthereumBlockId {
    Hash(EthereumHash),
    Number(EthereumBlockNumber),
}

#[derive(Serialize, Deserialize)] // used in config
#[derive(Clone, Eq, PartialEq, Encode, Decode, PassByCodec, RuntimeDebug, Types)]
pub struct EthereumBlock {
    #[serde(with = "ConstHexForm")]
    pub hash: EthereumHash,
    #[serde(with = "ConstHexForm")]
    pub parent_hash: EthereumHash,
    pub number: EthereumBlockNumber,
    #[serde(skip)]
    pub events: Vec<EthereumEvent>,
}

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum EthereumClientError {
    DecodeError,
    HttpIoError,
    HttpTimeout,
    HttpErrorCode(u16),
    InvalidUTF8,
    JsonParseError,
    NoResult,
}

#[derive(Deserialize, Serialize, RuntimeDebug, PartialEq)]
pub struct ResponseError {
    pub message: Option<String>,
    pub code: Option<i64>,
}

#[derive(Clone, Deserialize, RuntimeDebug, PartialEq)]
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

#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct GetLogsResponse {
    pub id: Option<u64>,
    pub result: Option<Vec<LogObject>>,
    pub error: Option<ResponseError>,
}

#[allow(non_snake_case)]
#[derive(Clone, Deserialize, Serialize, RuntimeDebug, PartialEq)]
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
    pub transactions: Option<Vec<String>>,
    pub transactionsRoot: Option<String>,
    pub uncles: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, RuntimeDebug, PartialEq)]
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

fn parse_error(data: &str) -> EthereumClientError {
    error!("Error Parsing: {}", data);
    EthereumClientError::JsonParseError
}

fn deserialize_get_logs_response(response: &str) -> Result<GetLogsResponse, EthereumClientError> {
    let result: serde_json::error::Result<GetLogsResponse> = serde_json::from_str(response);
    Ok(result.map_err(|_| parse_error(response))?)
}

fn deserialize_get_block_response(response: &str) -> Result<BlockResponse, EthereumClientError> {
    let result: serde_json::error::Result<BlockResponse> = serde_json::from_str(response);
    Ok(result.map_err(|_| parse_error(response))?)
}

fn deserialize_block_number_response(
    response: &str,
) -> Result<BlockNumberResponse, EthereumClientError> {
    let result: serde_json::error::Result<BlockNumberResponse> = serde_json::from_str(response);
    Ok(result.map_err(|_| parse_error(response))?)
}

pub fn encode_block_hash_hex(block_hash: EthereumHash) -> String {
    format!("0x{}", ::hex::encode(&block_hash))
}

pub fn encode_block_number_hex(block_number: EthereumBlockNumber) -> String {
    format!("{:#X}", block_number)
}

pub fn send_rpc(
    server: &str,
    method: serde_json::Value,
    params: Vec<serde_json::Value>,
) -> Result<String, EthereumClientError> {
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(ETH_FETCH_DEADLINE));
    let data = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id":1
    })
    .to_string();
    trace!("RPC: {}", &data);

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
        warn!("Unexpected status code: {}", response.code);
        return Err(EthereumClientError::HttpErrorCode(response.code));
    }

    let body = response.body().collect::<Vec<u8>>();

    // Create a str slice from the body.
    let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
        warn!("No UTF8 body");
        EthereumClientError::InvalidUTF8
    })?;
    trace!("RPC Response: {}", body_str.clone());

    Ok(String::from(body_str))
}

pub fn get_block(
    server: &str,
    eth_starport_address: &[u8; 20],
    block_id: EthereumBlockId,
) -> Result<EthereumBlock, EthereumClientError> {
    let block_obj = get_block_object(server, block_id.clone())?;
    let get_logs_params = vec![serde_json::json!({
        "address": format!("0x{}", ::hex::encode(&eth_starport_address[..])),
        "blockHash": &block_obj.hash
    })];
    debug!("get_logs_params: {:?}", get_logs_params.clone());
    let get_logs_response_str: String = send_rpc(server, "eth_getLogs".into(), get_logs_params)?;
    let get_logs_response = deserialize_get_logs_response(&get_logs_response_str)?;
    let event_objects = get_logs_response
        .result
        .ok_or_else(|| parse_error(&get_logs_response_str[..]))?;

    if event_objects.len() > 0 {
        info!(
            "Found {} events for Eth block {:?}",
            event_objects.len(),
            block_id
        );
    } else {
        debug!("Found no events for Eth block {:?}", block_id);
    }

    let mut events = Vec::with_capacity(event_objects.len());
    for ev_obj in event_objects {
        let topics = ev_obj
            .topics
            .ok_or_else(|| parse_error(&get_logs_response_str[..]))?;
        let data = ev_obj
            .data
            .ok_or_else(|| parse_error(&get_logs_response_str[..]))?;
        match events::decode_event(topics, data) {
            Ok(event) => events.push(event),
            Err(events::EventError::UnknownEventTopic(topic)) => {
                warn!("Skipping unrecognized topic {:?}", topic)
            }
            Err(err) => {
                error!("Failed to decode {:?}", err);
                return Err(EthereumClientError::DecodeError);
            }
        }
    }

    // note these error messages are imperfect as they don't show the broken data
    //  but also should never happen and not worth fixing for now
    Ok(EthereumBlock {
        hash: parse_word(block_obj.hash).ok_or_else(|| parse_error("bad hash"))?,
        parent_hash: parse_word(block_obj.parentHash)
            .ok_or_else(|| parse_error("bad parent hash"))?,
        number: parse_u64(block_obj.number).ok_or_else(|| parse_error("bad block number"))?,
        events,
    })
}

pub fn get_block_object(
    server: &str,
    block_id: EthereumBlockId,
) -> Result<BlockObject, EthereumClientError> {
    let response_str: String = match block_id {
        EthereumBlockId::Hash(hash) => {
            let params = vec![encode_block_hash_hex(hash).into(), false.into()];
            send_rpc(server, "eth_getBlockByHash".into(), params)?
        }

        EthereumBlockId::Number(number) => {
            let params = vec![encode_block_number_hex(number).into(), false.into()];
            send_rpc(server, "eth_getBlockByNumber".into(), params)?
        }
    };
    let response = deserialize_get_block_response(&response_str)?;
    response.result.ok_or(EthereumClientError::NoResult)
}

pub fn get_latest_block_number(server: &str) -> Result<u64, EthereumClientError> {
    let response_str: String = send_rpc(server, "eth_blockNumber".into(), vec![])?;
    let response = deserialize_block_number_response(&response_str)?;
    debug!("eth_blockNumber response: {:?}", response.result.clone());
    parse_u64(Some(response.result.ok_or(EthereumClientError::NoResult)?))
        .ok_or(EthereumClientError::JsonParseError)
}

#[cfg(test)]
mod tests {
    use crate::*;

    use sp_core::offchain::{testing, OffchainDbExt, OffchainWorkerExt};

    #[test]
    fn test_get_block() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = sp_io::TestExternalities::default();
        t.register_extension(OffchainDbExt::new(offchain.clone()));
        t.register_extension(OffchainWorkerExt::new(offchain));
        {
            let mut s = state.write();
            s.expect_request(
                testing::PendingRequest {
                    method: "POST".into(),
                    uri: "https://mainnet-eth.compound.finance".into(),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: br#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x506",false],"id":1}"#.to_vec(),
                    response: Some(br#"{"jsonrpc":"2.0","id":1,"result":{"difficulty":"0xb9e274f7969f5","extraData":"0x65746865726d696e652d657531","gasLimit":"0x7a121d","gasUsed":"0x781503","hash":"0x61314c1c6837e15e60c5b6732f092118dd25e3ec681f5e089b3a9ad2374e5a8a","logsBloom":"0x044410ea904e1020440110008000902200168801c81010301489212010002008080b0010004001b006040222c42004b001200408400500901889c908212040401020008d300010100198d10800100080027900254120000000530141030808140c299400162c0000d200204080008838240009002c020010400010101000481660200420a884b8020282204a00141ce10805004810800190180114180001b0001b1000020ac8040007000320b0480004018240891882a20080010281002c00000010102e0184210003010100438004202003080401000806204010000a42200104110100201200008081005001104002410140114a002010808c00200894c0c0","miner":"0xea674fdde714fd979de3edf0f56aa9716b898ec8","mixHash":"0xd733e12126a2155f0278c3987777eaca558a274b42d0396306dffb8fa6d21e76","nonce":"0x56a66f3802150748","number":"0x506","parentHash":"0x062e77dced431eb671a56839f96da912f68d841024665748d38cd3d6795961ea","receiptsRoot":"0x19ad317358916207491d4b64340153b924f4dda88fa8ef5dcb49090f234c00e7","sha3Uncles":"0xd21bed33f01dac18a3ee5538d1607ff2709d742eb4e13877cf66dcbed6c980f2","size":"0x5f50","stateRoot":"0x40b48fa241b8f9749af10a5dd1dfb8db245ba94cbb4969ab5c5b905a6adfe5f6","timestamp":"0x5aae89b9","totalDifficulty":"0xa91291ae5c752d4885","transactions":["0x94859e5d00b6bc572f877eaae906c0093eb22267d2d84d720ac90627fc63147c"],"transactionsRoot":"0xa46bb7bc06d4ad700df4100095fecd5a5af2994b6d1d24162ded673b7d485610","uncles":["0x5e7dde2e3811b5881a062c8b2ff7fd14687d79745e2384965d73a9df3fb0b4a8"]}}"#.to_vec()),
                    sent: true,
                    ..Default::default()
                });
            s.expect_request(
                testing::PendingRequest {
                    method: "POST".into(),
                    uri: "https://mainnet-eth.compound.finance".into(),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address":"0x3a275655586a049fe860be867d10cdae2ffc0f33","blockHash":"0x61314c1c6837e15e60c5b6732f092118dd25e3ec681f5e089b3a9ad2374e5a8a"}],"id":1}"#.to_vec(),
                    response: Some(br#"{"jsonrpc":"2.0","id":1,"result":[{"address":"0xd905abba1c5ea48c0598be9f3f8ae31290b58613","blockHash":"0xc94ceed3c8c68f09b1c7be28f594cc6fb01f9cdd7b68f3bf516cab9e89486fcf","blockNumber":"0x9928cb","data":"0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000006f05b59d3b2000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000","logIndex":"0x58","removed":false,"topics":["0xc459acef3ffe957663bb49d644b20d0c790bcb41573893752a72ba6f023b9386","0x000000000000000000000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","0x000000000000000000000000d3a38d4bd07b87e4516f30ee46cfe8ec4e8b73a4","0xd3a38d4bd07b87e4516f30ee46cfe8ec4e8b73a4000000000000000000000000"],"transactionHash":"0xbae1c242aea30e9ae20cb6c37e2f2d08982e31b42bf3d7dbde6466396abb360e","transactionIndex":"0x24"}]}"#.to_vec()),
                    sent: true,
                    ..Default::default()
                });
        }
        t.execute_with(|| {
            let result = get_block(
                "https://mainnet-eth.compound.finance",
                &[
                    58, 39, 86, 85, 88, 106, 4, 159, 232, 96, 190, 134, 125, 16, 205, 174, 47, 252,
                    15, 51,
                ],
                EthereumBlockId::Number(1286),
            );
            let block = result.unwrap();
            assert_eq!(
                block.hash,
                [
                    97, 49, 76, 28, 104, 55, 225, 94, 96, 197, 182, 115, 47, 9, 33, 24, 221, 37,
                    227, 236, 104, 31, 94, 8, 155, 58, 154, 210, 55, 78, 90, 138
                ]
            );
            assert_eq!(
                block.parent_hash,
                [
                    6, 46, 119, 220, 237, 67, 30, 182, 113, 165, 104, 57, 249, 109, 169, 18, 246,
                    141, 132, 16, 36, 102, 87, 72, 211, 140, 211, 214, 121, 89, 97, 234
                ]
            );
            assert_eq!(block.number, 1286);
            assert_eq!(
                block.events,
                vec![EthereumEvent::Lock {
                    asset: [
                        238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238,
                        238, 238, 238, 238, 238
                    ],
                    sender: [
                        211, 163, 141, 75, 208, 123, 135, 228, 81, 111, 48, 238, 70, 207, 232, 236,
                        78, 139, 115, 164
                    ],
                    chain: String::from("ETH"),
                    recipient: [
                        211, 163, 141, 75, 208, 123, 135, 228, 81, 111, 48, 238, 70, 207, 232, 236,
                        78, 139, 115, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                    ],
                    amount: 500000000000000000
                }]
            );
        });
    }

    #[test]
    fn test_get_latest_block_number() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = sp_io::TestExternalities::default();
        t.register_extension(OffchainDbExt::new(offchain.clone()));
        t.register_extension(OffchainWorkerExt::new(offchain));
        {
            let mut s = state.write();
            s.expect_request(testing::PendingRequest {
                method: "POST".into(),
                uri: "https://mainnet-eth.compound.finance".into(),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                body: br#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#
                    .to_vec(),
                response: Some(br#"{"jsonrpc":"2.0","id":1,"result":"0x123"}"#.to_vec()),
                sent: true,
                ..Default::default()
            });
        }
        t.execute_with(|| {
            let result = get_latest_block_number("https://mainnet-eth.compound.finance");
            assert_eq!(result, Ok(291));
        });
    }

    #[test]
    fn test_get_block_object() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = sp_io::TestExternalities::default();
        t.register_extension(OffchainDbExt::new(offchain.clone()));
        t.register_extension(OffchainWorkerExt::new(offchain));
        {
            let mut s = state.write();
            s.expect_request(
                testing::PendingRequest {
                    method: "POST".into(),
                    uri: "https://mainnet-eth.compound.finance".into(),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: br#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x506",false],"id":1}"#.to_vec(),
                    response: Some(br#"{"jsonrpc":"2.0","id":1,"result":{"difficulty":"0xb9e274f7969f5","extraData":"0x65746865726d696e652d657531","gasLimit":"0x7a121d","gasUsed":"0x781503","hash":"0x61314c1c6837e15e60c5b6732f092118dd25e3ec681f5e089b3a9ad2374e5a8a","logsBloom":"0x044410ea904e1020440110008000902200168801c81010301489212010002008080b0010004001b006040222c42004b001200408400500901889c908212040401020008d300010100198d10800100080027900254120000000530141030808140c299400162c0000d200204080008838240009002c020010400010101000481660200420a884b8020282204a00141ce10805004810800190180114180001b0001b1000020ac8040007000320b0480004018240891882a20080010281002c00000010102e0184210003010100438004202003080401000806204010000a42200104110100201200008081005001104002410140114a002010808c00200894c0c0","miner":"0xea674fdde714fd979de3edf0f56aa9716b898ec8","mixHash":"0xd733e12126a2155f0278c3987777eaca558a274b42d0396306dffb8fa6d21e76","nonce":"0x56a66f3802150748","number":"0x506","parentHash":"0x062e77dced431eb671a56839f96da912f68d841024665748d38cd3d6795961ea","receiptsRoot":"0x19ad317358916207491d4b64340153b924f4dda88fa8ef5dcb49090f234c00e7","sha3Uncles":"0xd21bed33f01dac18a3ee5538d1607ff2709d742eb4e13877cf66dcbed6c980f2","size":"0x5f50","stateRoot":"0x40b48fa241b8f9749af10a5dd1dfb8db245ba94cbb4969ab5c5b905a6adfe5f6","timestamp":"0x5aae89b9","totalDifficulty":"0xa91291ae5c752d4885","transactions":["0x94859e5d00b6bc572f877eaae906c0093eb22267d2d84d720ac90627fc63147c"],"transactionsRoot":"0xa46bb7bc06d4ad700df4100095fecd5a5af2994b6d1d24162ded673b7d485610","uncles":["0x5e7dde2e3811b5881a062c8b2ff7fd14687d79745e2384965d73a9df3fb0b4a8"]}}"#.to_vec()),
                    sent: true,
                    ..Default::default()
                });
        }
        t.execute_with(|| {
            let result = get_block_object(
                "https://mainnet-eth.compound.finance",
                EthereumBlockId::Number(0x506),
            );
            let block = result.unwrap();
            assert_eq!(block.difficulty, Some("0xb9e274f7969f5".into()));
            assert_eq!(block.number, Some("0x506".into()));
            assert_eq!(block.transactions.unwrap().len(), 1);
            assert_eq!(block.uncles.unwrap().len(), 1);
        });
    }

    #[test]
    fn test_deserialize_get_logs_response() {
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
        let expected = GetLogsResponse {
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
    fn test_deserialize_get_logs_response_error() {
        const RESPONSE: &str = r#"{
      "jsonrpc": "2.0",
      "id": 1,
      "error": {
        "code": -32005,
        "message": "query returned more than 10000 results"
      }
    }"#;
        let result = deserialize_get_logs_response(RESPONSE);
        let expected = GetLogsResponse {
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
    fn test_deserialize_get_logs_response_unexpected_input() {
        const RESPONSE: &str = r#"{"USD": 2}"#;
        let result = deserialize_get_logs_response(RESPONSE);
        let expected = GetLogsResponse {
            id: None,
            result: None,
            error: None,
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_encode_block_hash_hex() {
        assert_eq!(
            encode_block_hash_hex([0u8; 32]),
            "0x0000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn test_encode_block_number_hex() {
        assert_eq!(encode_block_number_hex(0xb27467 + 1), "0xB27468");
    }
}
