/// for now this will just focus on serialization and deserialization of payloads
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp_runtime::offchain::{http, Duration};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLogsParams {
    pub block_hash: Option<String>,
    pub topics: Option<Vec<String>>,
    pub address: Option<String>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
}

#[derive(Deserialize, Debug)]
pub struct ResponseError {
    pub message: Option<String>,
    pub code: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct ResponseWrapper<T> {
    pub id: Option<u64>,
    pub result: Option<Vec<T>>,
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

pub fn serialize_get_logs_request(params: GetLogsParams, id: u64) -> String {
    serialize_request("eth_getLogs", &params, id)
}

pub fn serialize_request<T>(method: &str, params: &T, id: u64) -> String
where
    T: ?Sized + Serialize,
{
    let to_serialize = json!({
        "jsonprc": "2.0",
        "method": method,
        "params": [ params ],
        "id": id
    });
    serde_json::to_string(&to_serialize).unwrap()
}

pub fn deserialize_get_logs_response(
    response: &str,
) -> serde_json::error::Result<ResponseWrapper<LogObject>> {
    serde_json::from_str(response)
}

fn extract_address(candidate: &ethabi::token::Token) -> anyhow::Result<ethabi::Address> {
    if let ethabi::token::Token::Address(address) = candidate {
        return Ok(*address);
    }
    Err(anyhow::anyhow!("candidate is not an address"))
}

fn extract_string(candidate: &ethabi::token::Token) -> anyhow::Result<String> {
    if let ethabi::token::Token::String(s) = candidate {
        return Ok(s.clone());
    }
    Err(anyhow::anyhow!("candidate is not an address"))
}

fn extract_uint(candidate: &ethabi::token::Token) -> anyhow::Result<ethabi::Uint> {
    if let ethabi::token::Token::Uint(u) = candidate {
        return Ok(*u);
    }
    Err(anyhow::anyhow!("candidate is not an address"))
}

#[derive(Debug)]
pub struct LockEvent {
    pub asset: ethabi::Address,
    pub holder: ethabi::Address,
    pub amount: ethabi::Uint,
}

#[derive(Debug)]
pub struct LockCashEvent {
    pub holder: ethabi::Address,
    pub amount: ethabi::Uint,
    pub yield_index: ethabi::Uint,
}

fn send_rpc(
    server: &'static str,
    method: &'static str,
    params: Vec<&str>,
) -> Result<String, http::Error> {
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
    let dat = format!(
        r#"{{"jsonrpc":"2.0","method":"{}","params":[{}],"id":1}}"#,
        method,
        params.join(",")
    );
    // debug::warn!("dat: {}", dat.clone());

    let request = http::Request::post(server, vec![dat]);

    let pending = request
        .deadline(deadline)
        .add_header("Content-Type", "application/json")
        .send()
        .map_err(|_| http::Error::IoError)?;

    let response = pending
        .try_wait(deadline)
        .map_err(|_| http::Error::DeadlineReached)??;

    if response.code != 200 {
        // debug::warn!("Unexpected status code: {}", response.code);
        return Err(http::Error::Unknown);
    }

    let body = response.body().collect::<Vec<u8>>();

    // Create a str slice from the body.
    let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
        // debug::warn!("No UTF8 body");
        http::Error::Unknown
    })?;

    Ok(String::from(body_str))
}

// this helped me https://codeburst.io/deep-dive-into-ethereum-logs-a8d2047c7371?gi=dfa340e5e3e5
fn decode_lock_events(data: String) -> anyhow::Result<LockEvent> {
    // the data are a hex encoded string starting with 0x
    if !data.starts_with("0x") {
        return Err(anyhow::anyhow!("missing 0x prefix"));
    }

    let to_decode: String = data.chars().skip(2).collect();
    let decoded = hex::decode(to_decode.as_bytes()).map_err(anyhow::Error::msg)?;

    // event Lock(address asset, address holder, uint amount);
    let abi_decoded = ethabi::decode(
        &vec![
            ethabi::param_type::ParamType::Address,   // asset
            ethabi::param_type::ParamType::Address,   // holder
            ethabi::param_type::ParamType::Uint(256), // amount
        ],
        &decoded,
    )
    .map_err(anyhow::Error::msg)?;

    // 3 args from above
    if abi_decoded.len() != 3 {
        return Err(anyhow::anyhow!(
            "length of decoded event data is not correct"
        ));
    }

    let asset = extract_address(&abi_decoded[0])?;
    let holder = extract_address(&abi_decoded[1])?;
    let amount = extract_uint(&abi_decoded[2])?;

    Ok(LockEvent {
        asset: asset,
        holder: holder,
        amount: amount,
    })
}

pub fn fetch_and_decode_lock_events() -> Result<Vec<LockEvent>, http::Error> {
    let body_str: String = send_rpc("https://kovan.infura.io/v3/a9f65788c3c4481da5f6f6820d4cf5c0",
    "eth_getLogs",
    vec!["{\"address\": \"0x3f861853B41e19D5BBe03363Bb2f50D191a723A2\", \"fromBlock\": \"0x146A47D\", \"toBlock\" : \"latest\", \"topics\":[\"0xddd0ae9ae645d3e7702ed6a55b29d04590c55af248d51c92c674638f3fb9d575\"]}"])?;

    let deserialized_body =
        deserialize_get_logs_response(&body_str).map_err(|_| http::Error::Unknown)?;

    if deserialized_body.error.is_some() {
        return Err(http::Error::Unknown);
    }

    let body_data = deserialized_body.result.ok_or(http::Error::Unknown)?;
    // debug::native::info!("Eth Starport found {} log result(s)", body_data.len());
    let mut lock_events: Vec<LockEvent> = Vec::new();

    for eth_log in body_data {
        if eth_log.block_hash.is_none()
            || eth_log.transaction_index.is_none()
            || eth_log.data.is_none()
        {
            // debug::native::info!("Missing critical field from eth log event");
            continue;
        }

        let deserialized = decode_lock_events(eth_log.data.unwrap());
        if deserialized.is_err() {
            // debug::native::info!("Could not deserialize lock event");
            continue;
        }

        // TODO add more block_hash and transaction_index field???
        // block_hash: eth_log.block_hash.unwrap(),
        // transaction_index: eth_log.transaction_index.unwrap(),
        lock_events.push(deserialized.unwrap());
    }

    Ok(lock_events)
}

// TODO enable tests back
#[cfg(test)]
mod tests {
    //     use crate::*;

    //     #[test]
    //     fn test_serialize_get_logs_request() {
    //         const BASIC_REQUEST_SERIALIZATION_EXPECTED: &str = r#"{"id":1,"jsonprc":"2.0","method":"eth_getLogs","params":[{"address":"address","blockHash":"block hash","fromBlock":1234,"toBlock":null,"topics":["topic1","topic2"]}]}"#;

    //         let req = GetLogsParams {
    //             block_hash: Some("block hash".to_owned()),
    //             topics: Some(vec!["topic1".to_owned(), "topic2".to_owned()]),
    //             address: Some("address".to_owned()),
    //             from_block: Some(1234),
    //             to_block: None,
    //         };
    //         let actual = serialize_get_logs_request(req, 1);
    //         assert_eq!(BASIC_REQUEST_SERIALIZATION_EXPECTED, actual);
    //     }

    //     #[test]
    //     fn test_deserialize_get_logs_request_happy_path() {
    //         const RESPONSE: &str = r#"{
    //   "jsonrpc": "2.0",
    //   "id": 1,
    //   "result": [
    //     {
    //       "address": "0x1a94fce7ef36bc90959e206ba569a12afbc91ca1",
    //       "blockHash": "0x7c5a35e9cb3e8ae0e221ab470abae9d446c3a5626ce6689fc777dcffcab52c70",
    //       "blockNumber": "0x5c29fb",
    //       "data": "0x0000000000000000000000003e3310720058c51f0de456e273c626cdd35065700000000000000000000000000000000000000000000000000000000000003185000000000000000000000000000000000000000000000000000000000000318200000000000000000000000000000000000000000000000000000000005c2a23",
    //       "logIndex": "0x1d",
    //       "removed": false,
    //       "topics": [
    //         "0x241ea03ca20251805084d27d4440371c34a0b85ff108f6bb5611248f73818b80"
    //       ],
    //       "transactionHash": "0x3dc91b98249fa9f2c5c37486a2427a3a7825be240c1c84961dfb3063d9c04d50",
    //       "transactionIndex": "0x1d"
    //     },
    //     {
    //       "address": "0x06012c8cf97bead5deae237070f9587f8e7a266d",
    //       "blockHash": "0x7c5a35e9cb3e8ae0e221ab470abae9d446c3a5626ce6689fc777dcffcab52c70",
    //       "blockNumber": "0x5c29fb",
    //       "data": "0x00000000000000000000000077ea137625739598666ded665953d26b3d8e374400000000000000000000000000000000000000000000000000000000000749ff00000000000000000000000000000000000000000000000000000000000a749d00000000000000000000000000000000000000000000000000000000005c2a0f",
    //       "logIndex": "0x57",
    //       "removed": false,
    //       "topics": [
    //         "0x241ea03ca20251805084d27d4440371c34a0b85ff108f6bb5611248f73818b80"
    //       ],
    //       "transactionHash": "0x788b1442414cb9c9a36dba2abe250763161a6f6395788a2e808f1b34e92beec1",
    //       "transactionIndex": "0x54"
    //     }
    //   ]
    // }"#;
    //         let actual = deserialize_get_logs_response(RESPONSE);
    //         assert!(actual.is_ok());
    //         let actual = actual.unwrap();
    //         // println!("{:?}", actual);
    //         assert!(actual.id.is_some());
    //         assert_eq!(actual.id.unwrap(), 1);
    //         assert!(actual.result.is_some());
    //         assert!(actual.error.is_none());
    //         // todo : assert all the fields, but i inspected it, it is working fine.....
    //     }

    //     #[test]
    //     fn test_deserialize_get_logs_request_error_path() {
    //         const RESPONSE: &str = r#"{
    //   "jsonrpc": "2.0",
    //   "id": 1,
    //   "error": {
    //     "code": -32005,
    //     "message": "query returned more than 10000 results"
    //   }
    // }"#;
    //         let actual = deserialize_get_logs_response(RESPONSE);
    //         assert!(actual.is_ok());
    //         let actual = actual.unwrap();
    //         // println!("{:?}", actual);
    //         assert!(actual.id.is_some());
    //         assert_eq!(actual.id.unwrap(), 1);
    //         assert!(actual.result.is_none());
    //         assert!(actual.error.is_some());
    //         // todo : assert all the fields, but i inspected it, it is working fine.....
    //     }

    //     #[test]
    //     fn test_deserialize_get_logs_request_totally_unexpected_input() {
    //         const RESPONSE: &str = r#"{"USD": 2}"#;
    //         let actual = deserialize_get_logs_response(RESPONSE);
    //         assert!(actual.is_ok());
    //         let actual = actual.unwrap();
    //         // println!("{:?}", actual);
    //         assert!(actual.id.is_none());
    //         assert!(actual.result.is_none());
    //         assert!(actual.error.is_none());
    //         // todo : assert all the fields, but i inspected it, it is working fine.....
    //     }

    //     #[test]
    //     fn test_abi_decoding() {
    //         // from https://kovan.etherscan.io/tx/0x1276fa72a2d8efec8e127dac6e57eb678e706cb4fbdd1b311bda75d2691b1941#eventlog
    //         const DATA_FIELD: &str = r#"0x000000000000000000000000513c1ff435eccedd0fda5edd2ad5e5461f0e872600000000000000000000000000000000000000000000000000000000000000a00000000000000000000000004f96fe3b7a6cf9725f59d353f723c1bdb64ca6aa00000000000000000000000000000000000000000000000000005af3107a4000000000000000000000000000000000000000000000000000000000005f7f5470000000000000000000000000000000000000000000000000000000000000002a30786463333145653137383432393233373946626232393634623342394334313234443846383943363000000000000000000000000000000000000000000000"#;

    //         let actual = decode_data_field_from_logs_response(String::from(DATA_FIELD));
    //         if actual.is_err() {
    //             println!("{}", actual.err().unwrap());
    //             assert!(false);
    //         }
    //     }
}
