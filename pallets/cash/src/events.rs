use crate::chains::eth;
use crate::log;
use crate::reason::Reason;
use crate::types::ValidatorIdentity;
use codec::alloc::string::String;
use codec::{Decode, Encode};
use our_std::{vec::Vec, RuntimeDebug};

extern crate ethereum_client;

#[derive(RuntimeDebug)]
pub struct EventInfo {
    pub latest_eth_block: u64,
    pub events: Vec<(ChainLogId, ChainLogEvent)>,
}

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainLogId {
    Eth(eth::BlockNumber, eth::LogIndex),
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainLogEvent {
    Eth(ethereum_client::EthereumLogEvent),
}

impl ChainLogId {
    pub fn show(&self) -> String {
        match self {
            ChainLogId::Eth(block_number, log_index) => {
                format!("Eth({},{})", block_number, log_index)
            }
        }
    }
}

/// Type for the status of an event on the queue.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EventState {
    Pending { signers: Vec<ValidatorIdentity> },
    Failed { reason: Reason },
    Done,
}

impl Default for EventState {
    fn default() -> Self {
        EventState::Pending { signers: vec![] }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EventError {
    EthRpcUrlMissing,
    EthRpcUrlInvalid,
    StarportAddressInvalid,
    EthereumClientError(ethereum_client::EthereumClientError),
    ErrorDecodingHex,
}

/// Fetch all latest Starport events for the offchain worker.
pub fn fetch_events(from_block: String) -> Result<EventInfo, EventError> {
    // Get a validator config from runtime-interfaces pallet
    // Use config to get an address for interacting with Ethereum JSON RPC client
    let config = runtime_interfaces::config_interface::get();
    let eth_rpc_url = runtime_interfaces::validator_config_interface::get_eth_rpc_url()
        .ok_or(EventError::EthRpcUrlMissing)?;
    let eth_rpc_url = String::from_utf8(eth_rpc_url).map_err(|_| EventError::EthRpcUrlInvalid)?;
    let eth_starport_address = String::from_utf8(config.get_eth_starport_address())
        .map_err(|_| EventError::StarportAddressInvalid)?;

    log!(
        "eth_rpc_url={}, starport_address={}",
        eth_rpc_url,
        eth_starport_address,
    );

    // Fetch the latest available ethereum block number
    let latest_eth_block = ethereum_client::fetch_latest_block(&eth_rpc_url)
        .map_err(EventError::EthereumClientError)?;

    // Build parameters set for fetching starport events
    let fetch_events_request = format!(
        r#"{{"address": "{}", "fromBlock": "{}", "toBlock": "{}"}}"#,
        eth_starport_address,
        from_block,
        encode_block_hex(latest_eth_block)
    );

    // Fetch events using ethereum_client
    let logs = ethereum_client::fetch_and_decode_logs(&eth_rpc_url, vec![&fetch_events_request])
        .map_err(EventError::EthereumClientError)?;

    let events = logs
        .into_iter()
        .map(|log| {
            (
                ChainLogId::Eth(log.block_number, log.log_index),
                ChainLogEvent::Eth(log),
            )
        })
        .collect();

    Ok(EventInfo {
        latest_eth_block,
        events,
    })
}

pub fn encode_block_hex(block_number: u64) -> String {
    format!("{:#X}", block_number)
}

fn hex_to_u32(hex_data: String) -> Result<u32, EventError> {
    let without_prefix = hex_data.trim_start_matches("0x");
    let u32_data =
        u32::from_str_radix(without_prefix, 16).map_err(|_| EventError::ErrorDecodingHex)?;
    Ok(u32_data)
}

#[cfg(test)]
pub mod tests {

    use crate::mock::*;
    use crate::*;
    use sp_core::offchain::testing;

    #[test]
    fn test_hex_to_u32_success() {
        let expected = 6008149;
        let actual = events::hex_to_u32("0x5bad55".to_string()).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_hex_to_u32_fail() {
        assert_eq!(events::hex_to_u32("".to_string()).is_err(), true)
    }

    pub fn get_mockup_http_calls(events_response: Vec<u8>) -> Vec<testing::PendingRequest> {
        // Set up config values
        let given_eth_starport_address: Vec<u8> =
            "0xbbde1662bC3ED16aA8C618c9833c801F3543B587".into();
        let config = runtime_interfaces::new_config(given_eth_starport_address.clone());
        runtime_interfaces::config_interface::set(config);
        runtime_interfaces::set_validator_config_dev_defaults();

        let given_eth_rpc_url =
            runtime_interfaces::validator_config_interface::get_eth_rpc_url().unwrap();
        return vec![
            testing::PendingRequest{
                method: "POST".into(),
                uri: String::from_utf8(given_eth_rpc_url.clone()).unwrap(),
                body: br#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#.to_vec(),
                response: Some(testdata::json_responses::BLOCK_NUMBER_RESPONSE.to_vec()),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                sent: true,
                ..Default::default()
            },
            testing::PendingRequest{
                method: "POST".into(),
                uri: String::from_utf8(given_eth_rpc_url.clone()).unwrap(),
                body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address": "0xbbde1662bC3ED16aA8C618c9833c801F3543B587", "fromBlock": "earliest", "toBlock": "0xB27467"}],"id":1}"#.to_vec(),
                response: Some(events_response.clone()),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                sent: true,
                ..Default::default()
            }
        ];
    }

    #[test]
    fn test_fetch_events_with_3_events() {
        let calls: Vec<testing::PendingRequest> =
            get_mockup_http_calls(testdata::json_responses::EVENTS_RESPONSE.to_vec());

        let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);
        t.execute_with(|| {
            let events_candidate = events::fetch_events("earliest".to_string());
            assert!(events_candidate.is_ok());
            let starport_info = events_candidate.unwrap();
            let latest_eth_block = starport_info.latest_eth_block;
            let mut events = starport_info.events;
            events.reverse(); // Since we'll be popping off the end

            assert_eq!(latest_eth_block, 11695207);
            assert_eq!(events.len(), 3);
            if let Some((_chain_log_id, ChainLogEvent::Eth(log))) = events.pop() {
                assert_eq!(
                    Ok(log.block_hash),
                    hex::decode("c1c0eb37b56923ad9e20fdb31ca882988d5217f7ca24b6297ca6ed700811cf23")
                        .unwrap()
                        .try_into()
                );
            } else {
                assert!(false);
            }

            if let Some((_chain_log_id, ChainLogEvent::Eth(log))) = events.pop() {
                assert_eq!(
                    Ok(log.block_hash),
                    hex::decode("a5c8024e699a5c30eb965e47b5157c06c76f3b726bff377a0a5333a561f25648")
                        .unwrap()
                        .try_into()
                );
            } else {
                assert!(false);
            }

            if let Some((_chain_log_id, ChainLogEvent::Eth(log))) = events.pop() {
                assert_eq!(
                    Ok(log.block_hash),
                    hex::decode("a4a96e957718e3a30b77a667f93978d8f438bdcd56ff03545f08c833d9a26687")
                        .unwrap()
                        .try_into()
                );
            } else {
                assert!(false);
            }
        });
    }

    #[test]
    fn test_fetch_events_with_no_events() {
        let calls: Vec<testing::PendingRequest> =
            get_mockup_http_calls(testdata::json_responses::NO_EVENTS_RESPONSE.to_vec());

        let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);
        t.execute_with(|| {
            let events_candidate = events::fetch_events("earliest".to_string());
            assert!(events_candidate.is_ok());
            let event_info = events_candidate.unwrap();
            let latest_eth_block = event_info.latest_eth_block;

            assert_eq!(latest_eth_block, 11695207);
            assert_eq!(event_info.events.len(), 0);
        });
    }

    #[test]
    fn test_encode_block_hex() {
        assert_eq!(events::encode_block_hex(0xb27467 + 1), "0xB27468");
    }
}
