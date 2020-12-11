use codec::alloc::string::String;
use frame_support::debug;

use sp_std::vec::Vec;

extern crate ethereum_client;

use crate::chains;

pub const ETH_STARPORT_ADDRESS: &str = "0xbbde1662bC3ED16aA8C618c9833c801F3543B587";
pub const LOCK_EVENT_TOPIC: &str =
    "0xec36c0364d931187a76cf66d7eee08fad0ec2e8b7458a8d8b26b36769d4d13f3";

#[derive(Debug)]
pub struct StarportInfo {
    pub latest_eth_block: String,
    pub lock_events: Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>,
}

/// Fetch all latest Starport events for the offchain worker.
pub fn fetch_events(eth_rpc_url: String, from_block: String) -> anyhow::Result<StarportInfo> {
    // Fetch the latest available ethereum block number
    let latest_eth_block = ethereum_client::fetch_latest_block(&eth_rpc_url).map_err(|e| {
        debug::native::error!("Error while fetching latest eth block number: {:?}", e);
        return anyhow::anyhow!("Fetching latest eth block failed: {:?}", e);
    })?;

    // Build parameters set for fetching starport `Lock` events
    let fetch_events_request = format!(
        r#"{{"address": "{}", "fromBlock": "{}", "toBlock": "{}", "topics":["{}"]}}"#,
        ETH_STARPORT_ADDRESS, from_block, latest_eth_block, LOCK_EVENT_TOPIC
    );

    // Fetch `Lock` events using ethereum_client
    let lock_events =
        ethereum_client::fetch_and_decode_events(&eth_rpc_url, vec![&fetch_events_request])
            .map_err(|e| {
                debug::native::error!("Error while fetching and decoding starport events: {:?}", e);
                return anyhow::anyhow!("Fetching and/or decoding starport events failed: {:?}", e);
            })?;

    Ok(StarportInfo {
        lock_events: lock_events,
        latest_eth_block: latest_eth_block,
    })
}

pub fn get_next_block_hex(block_num_hex: String) -> anyhow::Result<String> {
    let without_prefix = block_num_hex.trim_start_matches("0x");
    let block_num = u64::from_str_radix(without_prefix, 16)
        .map_err(|_| return anyhow::anyhow!("missing 0x prefix"))?;
    let next_block_num_hex = format!("{:#X}", block_num + 1);
    Ok(next_block_num_hex)
}

pub fn to_payload(
    event: &ethereum_client::LogEvent<ethereum_client::LockEvent>,
) -> anyhow::Result<chains::eth::Payload> {
    let block_number_without_prefix = &event.block_number.trim_start_matches("0x");
    let block_number: u32 = u32::from_str_radix(block_number_without_prefix, 16).map_err(|e| {
        debug::native::error!(
            "Error decoding an event's block_number {:?}: {:?}",
            &event.block_number,
            e
        );
        return anyhow::anyhow!(
            "Failed decoding an event's block_number {:?}: {:?}",
            &event.block_number,
            e
        );
    })?;
    let log_index_without_prefix = &event.log_index.trim_start_matches("0x");
    let log_index: u32 = u32::from_str_radix(log_index_without_prefix, 16).map_err(|e| {
        debug::native::error!("Error decoding an event's log_index: {:?}", e);
        return anyhow::anyhow!("Failed decoding an event's log_index: {:?}", e);
    })?;
    let event = chains::eth::Event {
        id: (block_number, log_index),
    };
    let payload: Vec<u8> = chains::eth::encode(&event);
    Ok(payload)
}
