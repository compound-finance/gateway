use codec::{Decode, Encode};
use hex_buffer_serde::{ConstHex, ConstHexForm};
use our_std::convert::TryInto;
use our_std::{debug, error, info, warn, Deserialize, RuntimeDebug, Serialize};
use sp_runtime::offchain::{http, Duration};
use sp_runtime_interface::pass_by::PassByCodec;
use types_derive::{type_alias, Types};

pub mod events;

pub use crate::events::FlowEvent;

#[type_alias]
pub type FlowBlockNumber = u64;

#[type_alias]
pub type FlowHash = [u8; 32];

const FLOW_FETCH_DEADLINE: u64 = 10_000;

#[derive(Serialize, Deserialize)] // used in config
#[derive(Clone, Eq, PartialEq, Encode, Decode, PassByCodec, RuntimeDebug, Types)]
#[allow(non_snake_case)]
pub struct FlowBlock {
    #[serde(with = "ConstHexForm")]
    pub blockId: FlowHash,
    #[serde(with = "ConstHexForm")]
    pub parentBlockId: FlowHash,
    pub height: FlowBlockNumber,
    #[serde(skip)]
    pub events: Vec<FlowEvent>,
}

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum FlowClientError {
    DecodeError,
    HttpIoError,
    HttpTimeout,
    HttpErrorCode(u16),
    InvalidUTF8,
    JsonParseError,
    NoResult,
    BadResponse,
}

// #[derive(Deserialize, RuntimeDebug, PartialEq)]
// pub struct ResponseError {
//     pub message: Option<String>,
//     pub code: Option<i64>,
// }

#[derive(Clone, Deserialize, RuntimeDebug, PartialEq)]
#[allow(non_snake_case)]
pub struct LogObject {
    blockId: Option<String>,
    blockHeight: Option<FlowBlockNumber>,
    transactionId: Option<String>,
    transactionIndex: Option<u8>,
    eventIndex: Option<u8>,
    topic: Option<String>,
    data: Option<String>,
}

#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct GetLogsResponse {
    pub result: Option<Vec<LogObject>>,
    // pub error: Option<ResponseError>,
}

#[allow(non_snake_case)]
#[derive(Clone, Deserialize, RuntimeDebug, PartialEq)]
pub struct BlockObject {
    pub blockId: Option<String>,
    pub parentBlockId: Option<String>,
    pub height: Option<FlowBlockNumber>,
    pub timestamp: Option<String>,
}

#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct BlockResponse {
    pub result: Option<BlockObject>,
    // pub error: Option<ResponseError>,
}

#[derive(Deserialize, RuntimeDebug, PartialEq)]
pub struct BlockNumberResponse {
    pub result: Option<u64>,
    // pub error: Option<ResponseError>,
}

fn deserialize_get_logs_response(response: &str) -> Result<GetLogsResponse, FlowClientError> {
    let result: serde_json::error::Result<GetLogsResponse> = serde_json::from_str(response);
    Ok(result.map_err(|_| FlowClientError::BadResponse)?)
}

// TODO think about error field and introducing back parse_error method
fn deserialize_get_block_by_number_response(
    response: &str,
) -> Result<BlockResponse, FlowClientError> {
    let result: serde_json::error::Result<BlockResponse> = serde_json::from_str(response);
    Ok(result.map_err(|_| FlowClientError::BadResponse)?)
}

fn deserialize_block_number_response(
    response: &str,
) -> Result<BlockNumberResponse, FlowClientError> {
    let result: serde_json::error::Result<BlockNumberResponse> = serde_json::from_str(response);
    Ok(result.map_err(|_| FlowClientError::BadResponse)?)
}

// TODO think about optional data here???
pub fn send_request(server: &str, path: &str, data: &str) -> Result<String, FlowClientError> {
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(FLOW_FETCH_DEADLINE));
    let request_url = format!("{}/{}", server, path);

    debug!("Request {}, with data {}", request_url, data);

    let request = http::Request::post(&request_url, vec![data.to_string()]);

    let pending = request
        .deadline(deadline)
        .add_header("Content-Type", "application/json")
        .send()
        .map_err(|_| FlowClientError::HttpIoError)?;

    let response = pending
        .try_wait(deadline)
        .map_err(|_| FlowClientError::HttpTimeout)?
        .map_err(|_| FlowClientError::HttpTimeout)?;

    if response.code != 200 {
        warn!("Unexpected status code: {}", response.code);
        return Err(FlowClientError::HttpErrorCode(response.code));
    }

    let body = response.body().collect::<Vec<u8>>();

    // Create a str slice from the body.
    let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
        warn!("No UTF8 body");
        FlowClientError::InvalidUTF8
    })?;
    debug!("Request Response: {}", body_str.clone());

    Ok(String::from(body_str))
}

pub fn get_block(
    server: &str,
    flow_starport_address: &str,
    block_num: FlowBlockNumber,
    topic: &str, // Lock
) -> Result<FlowBlock, FlowClientError> {
    debug!(
        "flow_starport_address: {:X?}, block_num {:?}, topic {:?}",
        flow_starport_address, block_num, topic
    );
    let block_obj = get_block_object(server, block_num)?;
    let get_logs_params = serde_json::json!({
        "topic": format!("A.{}.Starport.{}", flow_starport_address, topic),
        "startHeight": block_num,
        "endHeight": block_num,
    });
    debug!("get_logs_params: {:?}", get_logs_params.clone());
    let get_logs_response_str: String =
        send_request(server, "events", &get_logs_params.to_string())?;
    let get_logs_response = deserialize_get_logs_response(&get_logs_response_str)?;
    let event_objects = get_logs_response
        .result
        .ok_or_else(|| FlowClientError::BadResponse)?;

    if event_objects.len() > 0 {
        info!(
            "Found {} events @ Flow Starport {}",
            event_objects.len(),
            flow_starport_address
        );
    }

    let mut events = Vec::with_capacity(event_objects.len());
    for ev_obj in event_objects {
        let data = ev_obj.data.ok_or_else(|| FlowClientError::BadResponse)?;
        match events::decode_event(topic, &data) {
            Ok(event) => events.push(event),
            Err(events::EventError::UnknownEventTopic(topic)) => {
                warn!("Skipping unrecognized topic {:?}", topic)
            }
            Err(err) => {
                error!("Failed to decode {:?}", err);
                return Err(FlowClientError::DecodeError);
            }
        }
    }

    Ok(FlowBlock {
        blockId: hex::decode(block_obj.blockId.ok_or(FlowClientError::DecodeError)?)
            .map_err(|_| FlowClientError::DecodeError)?
            .try_into()
            .map_err(|_| FlowClientError::DecodeError)?,
        parentBlockId: hex::decode(
            block_obj
                .parentBlockId
                .ok_or(FlowClientError::DecodeError)?,
        )
        .map_err(|_| FlowClientError::DecodeError)?
        .try_into()
        .map_err(|_| FlowClientError::DecodeError)?,
        height: block_obj.height.ok_or(FlowClientError::DecodeError)?,
        events,
    })
}

pub fn get_block_object(
    server: &str,
    block_num: FlowBlockNumber,
) -> Result<BlockObject, FlowClientError> {
    let get_block_params = serde_json::json!({
        "height": block_num,
    });
    let response_str: String = send_request(server, "block", &get_block_params.to_string())?;
    debug!("get block object {}", response_str);
    let response = deserialize_get_block_by_number_response(&response_str)?;
    response.result.ok_or(FlowClientError::NoResult)
}

pub fn get_latest_block_number(server: &str) -> Result<FlowBlockNumber, FlowClientError> {
    let response_str: String = send_request(server, "latest_block_number", "{}")?;
    let response = deserialize_block_number_response(&response_str)?;
    debug!("eth_blockNumber response: {:?}", response.result.clone());
    Ok(response.result.ok_or(FlowClientError::NoResult)?)
}

// TODO add error tests
#[cfg(test)]
mod tests {
    use crate::*;

    use sp_core::offchain::{testing, OffchainDbExt, OffchainWorkerExt};

    #[test]
    fn test_flow_get_block() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = sp_io::TestExternalities::default();
        t.register_extension(OffchainDbExt::new(offchain.clone()));
        t.register_extension(OffchainWorkerExt::new(offchain));
        {
            let mut s = state.write();
            s.expect_request(
                testing::PendingRequest {
                    method: "GET".into(),
                    uri: "https://mainnet-flow-fetcher/block".into(),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: br#"{"height":34944396}"#.to_vec(),
                    response: Some(br#"{"result":{"blockId":"4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289","parentBlockId":"35afa24cc7ea92585b11a4e220c8226b5613556c8deb9f24d008acbdcf24c80d","height":34944396,"timestamp":"2021-06-09 21:16:57.510218679 +0000 UTC"}}"#.to_vec()),
                    sent: true,
                    ..Default::default()
            });
            s.expect_request(
                testing::PendingRequest {
                    method: "GET".into(),
                    uri: "https://mainnet-flow-fetcher/events".into(),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: br#"{"topic":"A.c8873a26b148ed14.Starport.Lock","startHeight":34944396,"endHeight":34944396}"#.to_vec(),
                    response: Some(br#"{"result":[{"blockId":"4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289","blockHeight":34944396,"transactionId":"f4d331583dd5ddc1e57e72ca02197d7ff365bde7b2ca9f3114d8c44d248d1c6c","transactionIndex":0,"eventIndex":2,"topic":"A.c8873a26b148ed14.Starport.Lock","data":"{\"asset\":\"FLOW\",\"recipient\":\"fc6346ab93540e97\",\"amount\":1000000000}"}]}"#.to_vec()),
                    sent: true,
                    ..Default::default()
                });
        }
        t.execute_with(|| {
            let result = get_block(
                "https://mainnet-flow-fetcher",
                "c8873a26b148ed14", // Starport address
                34944396,
                "Lock",
            );
            let block = result.unwrap();
            assert_eq!(
                block.blockId,
                "4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289"
            );
            assert_eq!(
                block.parentBlockId,
                "35afa24cc7ea92585b11a4e220c8226b5613556c8deb9f24d008acbdcf24c80d"
            );
            assert_eq!(block.height, 34944396);

            assert_eq!(
                block.events,
                vec![FlowEvent::Lock {
                    asset: String::from("FLOW"),
                    recipient: String::from("fc6346ab93540e97"),
                    amount: 1000000000
                }]
            );
        });
    }

    #[test]
    fn test_flow_get_latest_block_number() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = sp_io::TestExternalities::default();
        t.register_extension(OffchainDbExt::new(offchain.clone()));
        t.register_extension(OffchainWorkerExt::new(offchain));
        {
            let mut s = state.write();
            s.expect_request(testing::PendingRequest {
                method: "GET".into(),
                uri: "https://mainnet-flow-fetcher/latest_block_number".into(),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                body: br#"{}"#.to_vec(),
                response: Some(br#"{"result": 35705489}"#.to_vec()),
                sent: true,
                ..Default::default()
            });
        }
        t.execute_with(|| {
            let result = get_latest_block_number("https://mainnet-flow-fetcher");
            assert_eq!(result, Ok(35705489));
        });
    }

    #[test]
    fn test_flow_get_block_object() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = sp_io::TestExternalities::default();
        t.register_extension(OffchainDbExt::new(offchain.clone()));
        t.register_extension(OffchainWorkerExt::new(offchain));
        {
            let mut s = state.write();
            s.expect_request(
                testing::PendingRequest {
                    method: "GET".into(),
                    uri: "https://mainnet-flow-fetcher/block".into(),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: br#"{"height":34944396}"#.to_vec(),
                    response: Some(br#"{"result":{"blockId":"4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289","parentBlockId":"35afa24cc7ea92585b11a4e220c8226b5613556c8deb9f24d008acbdcf24c80d","height":34944396,"timestamp":"2021-06-09 21:16:57.510218679 +0000 UTC"}}"#.to_vec()),
                    sent: true,
                    ..Default::default()
                });
        }
        t.execute_with(|| {
            let result = get_block_object("https://mainnet-flow-fetcher", 34944396);
            let block = result.unwrap();
            assert_eq!(
                block.blockId,
                Some("4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289".into())
            );
            assert_eq!(
                block.parentBlockId,
                Some("35afa24cc7ea92585b11a4e220c8226b5613556c8deb9f24d008acbdcf24c80d".into())
            );
            assert_eq!(block.height, Some(34944396));
            assert_eq!(
                block.timestamp,
                Some("2021-06-09 21:16:57.510218679 +0000 UTC".into())
            );
        });
    }

    #[test]
    fn test_flow_deserialize_get_logs_response() {
        const RESPONSE: &str = r#"{
            "result":[
                {
                    "blockId":"4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289",
                    "blockHeight":34944396,
                    "transactionId":"f4d331583dd5ddc1e57e72ca02197d7ff365bde7b2ca9f3114d8c44d248d1c6c",
                    "transactionIndex":0,
                    "eventIndex":2,
                    "topic":"A.c8873a26b148ed14.Starport.Lock",
                    "data":"{\"asset\":\"FLOW\",\"recipient\":\"fc6346ab93540e97\",\"amount\":1000000000}"
                }
        ]
    }"#;
        let result = deserialize_get_logs_response(RESPONSE);
        let expected = GetLogsResponse {
            result: Some(vec![LogObject {
                blockId: Some(String::from(
                    "4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289",
                )),
                blockHeight: Some(34944396),
                transactionId: Some(String::from(
                    "f4d331583dd5ddc1e57e72ca02197d7ff365bde7b2ca9f3114d8c44d248d1c6c",
                )),
                transactionIndex: Some(0),
                eventIndex: Some(2),
                topic: Some(String::from("A.c8873a26b148ed14.Starport.Lock")),
                data: Some(String::from(
                    "{\"asset\":\"FLOW\",\"recipient\":\"fc6346ab93540e97\",\"amount\":1000000000}",
                )),
            }]),
            // error: None,
        };
        assert_eq!(result.unwrap(), expected)
    }

    #[test]
    fn test_flow_deserialize_get_block_by_number_response() {
        const RESPONSE: &str = r#"{
            "result":{
                "blockId":"4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289",
                "parentBlockId":"35afa24cc7ea92585b11a4e220c8226b5613556c8deb9f24d008acbdcf24c80d",
                "height":34944396,
                "timestamp":"2021-06-09 21:16:57.510218679 +0000 UTC"
            }
        }"#;
        let result = deserialize_get_block_by_number_response(RESPONSE);
        let expected = BlockResponse {
            result: Some(BlockObject {
                blockId: Some(String::from(
                    "4ac2583773d9d3e994e76ac2432f6a3b3641410894c5ff7616f6ce244b35b289",
                )),
                parentBlockId: Some(String::from(
                    "35afa24cc7ea92585b11a4e220c8226b5613556c8deb9f24d008acbdcf24c80d",
                )),
                height: Some(34944396),
                timestamp: Some(String::from("2021-06-09 21:16:57.510218679 +0000 UTC")),
            }),
            // error: None,
        };
        assert_eq!(result.unwrap(), expected)
    }

    #[test]
    fn test_flow_deserialize_block_number_response() {
        const RESPONSE: &str = r#"{"result": 35705489}"#;
        let result = deserialize_block_number_response(RESPONSE);
        let expected = BlockNumberResponse {
            result: Some(35705489),
            // error: None,
        };
        assert_eq!(result.unwrap(), expected)
    }
}
