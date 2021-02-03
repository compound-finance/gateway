use crate::{chains, error, log};
use codec::alloc::string::String;
use codec::Encode;
use our_std::{vec::Vec, RuntimeDebug};

extern crate ethereum_client;

// XXX why starport?
#[derive(RuntimeDebug)]
pub struct StarportInfo {
    pub latest_eth_block: String,
    pub lock_events: Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>,
}

/// Fetch all latest Starport events for the offchain worker.
pub fn fetch_events(from_block: String) -> anyhow::Result<StarportInfo> {
    // Get a validator config from runtime-interfaces pallet
    // Use config to get an address for interacting with Ethereum JSON RPC client
    let config = runtime_interfaces::config_interface::get();
    let eth_rpc_url = runtime_interfaces::validator_config_interface::get_eth_rpc_url()
        .ok_or_else(|| anyhow::anyhow!("Error reading `eth_rpc_url` from config ETH_RPC_URL environment variable is not set"))?;
    let eth_rpc_url = String::from_utf8(eth_rpc_url)
        .map_err(|e| anyhow::anyhow!("Error reading `eth_rpc_url` from config {:?}", e))?;
    let eth_starport_address = String::from_utf8(config.get_eth_starport_address())
        .map_err(|e| anyhow::anyhow!("Error reading `eth_starport_address` from config {:?}", e))?;
    let eth_lock_event_topic = String::from_utf8(config.get_eth_lock_event_topic())
        .map_err(|e| anyhow::anyhow!("Error reading `eth_lock_event_topic` from config {:?}", e))?;

    log!(
        "eth_rpc_url={}, starport_address={}, lock_event_topic={}",
        eth_rpc_url,
        eth_starport_address,
        eth_lock_event_topic
    );

    // Fetch the latest available ethereum block number
    let latest_eth_block = ethereum_client::fetch_latest_block(&eth_rpc_url).map_err(|e| {
        error!("fetch_events error: {:?}", e);
        return anyhow::anyhow!("Fetching latest eth block failed: {:?}", e);
    })?;

    // Build parameters set for fetching starport `Lock` events
    let fetch_events_request = format!(
        r#"{{"address": "{}", "fromBlock": "{}", "toBlock": "{}", "topics":["{}"]}}"#,
        eth_starport_address, from_block, latest_eth_block, eth_lock_event_topic
    );

    // Fetch `Lock` events using ethereum_client
    let lock_events =
        ethereum_client::fetch_and_decode_events(&eth_rpc_url, vec![&fetch_events_request])
            .map_err(|e| {
                error!("fetch_and_decode_events error: {:?}", e);
                return anyhow::anyhow!("Fetching and/or decoding starport events failed: {:?}", e);
            })?;

    Ok(StarportInfo {
        lock_events: lock_events,
        latest_eth_block: latest_eth_block,
    })
}

pub fn get_next_block_hex(block_num_hex: String) -> anyhow::Result<String> {
    let block_num = hex_to_u32(block_num_hex)?;
    let next_block_num_hex = format!("{:#X}", block_num + 1);
    Ok(next_block_num_hex)
}

// XXX JF: why just lock event? also can we just use builtin encoding?
pub fn to_lock_event_payload(
    log_event: &ethereum_client::LogEvent<ethereum_client::LockEvent>,
) -> anyhow::Result<Vec<u8>> {
    let block_number: u32 = hex_to_u32(log_event.block_number.clone())?;
    let log_index: u32 = hex_to_u32(log_event.log_index.clone())?;

    let asset_address: [u8; 20] = *log_event.event.asset.as_fixed_bytes();
    let holder_address: [u8; 20] = *log_event.event.holder.as_fixed_bytes();

    let event = chains::eth::Event {
        id: (block_number, log_index),
        data: chains::eth::EventData::Lock {
            asset: asset_address,
            holder: holder_address,
            amount: log_event.event.amount.as_u128(),
        },
    };
    let payload: Vec<u8> = event.encode();
    Ok(payload)
}

fn hex_to_u32(hex_data: String) -> anyhow::Result<u32> {
    let without_prefix = hex_data.trim_start_matches("0x");
    let u32_data = u32::from_str_radix(without_prefix, 16).map_err(|e| {
        error!("hex_to_u32 error {:?}", e);
        return anyhow::anyhow!(
            "Error decoding number in hex format {:?}: {:?}",
            without_prefix,
            e
        );
    })?;
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

    #[test]
    fn test_to_lock_event_payload_success() {
        const DATA_FIELD: &str = r#"0x000000000000000000000000d87ba7a50b2e7e660f678a895e4b72e7cb4ccd9c000000000000000000000000b819706e897eacf235cdb5048962bd65873202c400000000000000000000000000000000000000000000000000000000018cba80"#;
        let lock_event: ethereum_client::LockEvent =
            ethereum_client::DecodableEvent::new(DATA_FIELD.to_string());

        let event = ethereum_client::LogEvent {
            block_hash: "0xc1c0eb37b56923ad9e20fdb31ca882988d5217f7ca24b6297ca6ed700811cf23"
                .to_string(),
            block_number: "0x3adf2f".to_string(),
            transaction_index: "0x0".to_string(),
            log_index: "0x0".to_string(),
            event: lock_event,
        };

        let expected = [
            47, 223, 58, 0, 0, 0, 0, 0, 0, 216, 123, 167, 165, 11, 46, 126, 102, 15, 103, 138, 137,
            94, 75, 114, 231, 203, 76, 205, 156, 184, 25, 112, 110, 137, 126, 172, 242, 53, 205,
            181, 4, 137, 98, 189, 101, 135, 50, 2, 196, 128, 186, 140, 1, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ];
        let actual = events::to_lock_event_payload(&event).unwrap();
        assert_eq!(actual, expected);
    }

    fn get_mockup_http_calls(events_response: Vec<u8>) -> Vec<testing::PendingRequest> {
        // Set up config values
        let given_eth_starport_address: Vec<u8> =
            "0xbbde1662bC3ED16aA8C618c9833c801F3543B587".into();
        let given_eth_lock_event_topic: Vec<u8> =
            "0xec36c0364d931187a76cf66d7eee08fad0ec2e8b7458a8d8b26b36769d4d13f3".into();
        let config = runtime_interfaces::new_config(
            given_eth_starport_address.clone(),
            given_eth_lock_event_topic.clone(),
        );
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
                body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address": "0xbbde1662bC3ED16aA8C618c9833c801F3543B587", "fromBlock": "earliest", "toBlock": "0xb27467", "topics":["0xec36c0364d931187a76cf66d7eee08fad0ec2e8b7458a8d8b26b36769d4d13f3"]}],"id":1}"#.to_vec(),
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

        new_test_ext_with_http_calls(calls).execute_with(|| {
            let events_candidate = events::fetch_events("earliest".to_string());
            assert!(events_candidate.is_ok());
            let starport_info = events_candidate.unwrap();
            let latest_eth_block = starport_info.latest_eth_block;
            let lock_events = starport_info.lock_events;

            assert_eq!(latest_eth_block, "0xb27467");
            assert_eq!(lock_events.len(), 3);
            assert_eq!(
                lock_events[0].block_hash,
                "0xc1c0eb37b56923ad9e20fdb31ca882988d5217f7ca24b6297ca6ed700811cf23"
            );
            assert_eq!(
                lock_events[1].block_hash,
                "0xa5c8024e699a5c30eb965e47b5157c06c76f3b726bff377a0a5333a561f25648"
            );
            assert_eq!(
                lock_events[2].block_hash,
                "0xa4a96e957718e3a30b77a667f93978d8f438bdcd56ff03545f08c833d9a26687"
            );
        });
    }

    #[test]
    fn test_fetch_events_with_no_events() {
        let calls: Vec<testing::PendingRequest> =
            get_mockup_http_calls(testdata::json_responses::NO_EVENTS_RESPONSE.to_vec());

        new_test_ext_with_http_calls(calls).execute_with(|| {
            let events_candidate = events::fetch_events("earliest".to_string());
            assert!(events_candidate.is_ok());
            let starport_info = events_candidate.unwrap();
            let latest_eth_block = starport_info.latest_eth_block;
            let lock_events = starport_info.lock_events;

            assert_eq!(latest_eth_block, "0xb27467");
            assert_eq!(lock_events.len(), 0);
        });
    }

    #[test]
    fn test_get_next_block_hex() {
        let actual = events::get_next_block_hex("0xb27467".into());
        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), "0xB27468");
    }
}
