use frame_support::debug;
/// for now this will just focus on serialization and deserialization of payloads
use serde::Deserialize;
use sp_runtime::offchain::{http, Duration};

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

fn extract_address(candidate: &ethabi::token::Token) -> anyhow::Result<ethabi::Address> {
    if let ethabi::token::Token::Address(address) = candidate {
        return Ok(*address);
    }
    Err(anyhow::anyhow!("candidate is not an address"))
}

// TODO enable back if needed later
// fn extract_string(candidate: &ethabi::token::Token) -> anyhow::Result<String> {
//     if let ethabi::token::Token::String(s) = candidate {
//         return Ok(s.clone());
//     }
//     Err(anyhow::anyhow!("candidate is not a string"))
// }

pub fn extract_uint(candidate: &ethabi::token::Token) -> anyhow::Result<ethabi::Uint> {
    if let ethabi::token::Token::Uint(u) = candidate {
        return Ok(*u);
    }
    Err(anyhow::anyhow!("candidate is not an uint"))
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

#[derive(Debug)]
pub struct LogEvent<T: DecodableEvent> {
    pub block_hash: String,
    pub block_number: String,
    pub transaction_index: String,
    pub log_index: String,
    pub event: T,
}

pub trait DecodableEvent {
    fn new(data: String) -> Self;
}

impl DecodableEvent for LockEvent {
    fn new(data: String) -> LockEvent {
        let abi_decoded = decode_events(
            data,
            vec![
                ethabi::param_type::ParamType::Address,   // asset
                ethabi::param_type::ParamType::Address,   // holder
                ethabi::param_type::ParamType::Uint(256), // amount
            ],
        );

        let decoded = abi_decoded.unwrap();
        let asset = extract_address(&decoded[0]).unwrap();
        let holder = extract_address(&decoded[1]).unwrap();
        let amount = extract_uint(&decoded[2]).unwrap();

        return LockEvent {
            asset: asset,
            holder: holder,
            amount: amount,
        };
    }
}

// TODO add implementation of DecodableEvent for LockCashEvent

fn send_rpc(server: &str, method: &'static str, params: Vec<&str>) -> Result<String, http::Error> {
    // TODO - move 2_000 to config???
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
    let data = format!(
        r#"{{"jsonrpc":"2.0","method":"{}","params":[{}],"id":1}}"#,
        method,
        params.join(",")
    );
    debug::native::info!("Data for send_rpc: {}", data.clone());

    let request = http::Request::post(server, vec![data]);

    let pending = request
        .deadline(deadline)
        .add_header("Content-Type", "application/json")
        .send()
        .map_err(|_| http::Error::IoError)?;

    let response = pending
        .try_wait(deadline)
        .map_err(|_| http::Error::DeadlineReached)??;

    if response.code != 200 {
        debug::warn!("Unexpected status code: {}", response.code);
        return Err(http::Error::Unknown);
    }

    let body = response.body().collect::<Vec<u8>>();

    // Create a str slice from the body.
    let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
        debug::warn!("No UTF8 body");
        http::Error::Unknown
    })?;

    Ok(String::from(body_str))
}

// this helped me https://codeburst.io/deep-dive-into-ethereum-logs-a8d2047c7371?gi=dfa340e5e3e5
fn decode_events(
    data: String,
    types: Vec<ethabi::param_type::ParamType>,
) -> anyhow::Result<Vec<ethabi::token::Token>> {
    // the data are a hex encoded string starting with 0x
    if !data.starts_with("0x") {
        return Err(anyhow::anyhow!("missing 0x prefix"));
    }

    let to_decode: String = data.chars().skip(2).collect();
    let decoded = hex::decode(to_decode.as_bytes()).map_err(anyhow::Error::msg)?;

    // event Lock(address asset, address holder, uint amount);
    let abi_decoded = ethabi::decode(&types[..], &decoded).map_err(anyhow::Error::msg)?;

    // Check that lengths are the same
    if abi_decoded.len() != types.len() {
        return Err(anyhow::anyhow!(
            "length of decoded event data is not correct"
        ));
    }

    Ok(abi_decoded)
}

pub fn fetch_and_decode_events<T: DecodableEvent>(
    server: &str,
    params: Vec<&str>,
) -> Result<Vec<LogEvent<T>>, http::Error> {
    let body_str: String = send_rpc(server, "eth_getLogs", params)?;
    let deserialized_body =
        deserialize_get_logs_response(&body_str).map_err(|_| http::Error::Unknown)?;

    let body_data = deserialized_body.result.ok_or(http::Error::Unknown)?;
    debug::native::info!("Eth Starport found {} log result(s)", body_data.len());
    let mut log_events: Vec<LogEvent<T>> = Vec::new();

    for eth_log in body_data {
        if eth_log.block_hash.is_none()
            || eth_log.transaction_index.is_none()
            || eth_log.data.is_none()
            || eth_log.block_number.is_none()
            || eth_log.log_index.is_none()
        {
            debug::native::info!("Missing critical field from eth log event");
            continue;
        }

        let lock_event = DecodableEvent::new(eth_log.data.ok_or(http::Error::Unknown)?);
        log_events.push(LogEvent {
            block_hash: eth_log.block_hash.ok_or(http::Error::Unknown)?,
            block_number: eth_log.block_number.ok_or(http::Error::Unknown)?,
            transaction_index: eth_log.transaction_index.ok_or(http::Error::Unknown)?,
            log_index: eth_log.log_index.ok_or(http::Error::Unknown)?,
            event: lock_event,
        });
    }

    Ok(log_events)
}

pub fn fetch_latest_block(server: &str) -> Result<String, http::Error> {
    let body_str: String = send_rpc(server, "eth_blockNumber", vec![])?;
    let deserialized_body =
        deserialize_get_block_number_response(&body_str).map_err(|_| http::Error::Unknown)?;

    let block_number = deserialized_body.result.ok_or(http::Error::Unknown)?;
    return Ok(block_number);
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

    #[test]
    fn test_decode_events() {
        // from https://kovan.etherscan.io/tx/0x1276fa72a2d8efec8e127dac6e57eb678e706cb4fbdd1b311bda75d2691b1941#eventlog
        const DATA_FIELD: &str = r#"0x000000000000000000000000513c1ff435eccedd0fda5edd2ad5e5461f0e872600000000000000000000000000000000000000000000000000000000000000a00000000000000000000000004f96fe3b7a6cf9725f59d353f723c1bdb64ca6aa00000000000000000000000000000000000000000000000000005af3107a4000000000000000000000000000000000000000000000000000000000005f7f5470000000000000000000000000000000000000000000000000000000000000002a30786463333145653137383432393233373946626232393634623342394334313234443846383943363000000000000000000000000000000000000000000000"#;
        let actual = decode_events(
            String::from(DATA_FIELD),
            vec![
                ethabi::param_type::ParamType::Address,
                ethabi::param_type::ParamType::Address,
                ethabi::param_type::ParamType::Uint(256),
            ],
        );
        if actual.is_err() {
            println!("{}", actual.err().unwrap());
            assert!(false);
        }
    }
}
