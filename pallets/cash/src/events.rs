use codec::alloc::string::String;
use frame_support::debug;

use sp_std::vec::Vec;

extern crate ethereum_client;
use codec::Encode;

use crate::chains;

#[derive(Debug)]
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

    // Fetch the latest available ethereum block number
    let latest_eth_block = ethereum_client::fetch_latest_block(&eth_rpc_url).map_err(|e| {
        debug::native::error!("fetch_events error: {:?}", e);
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
                debug::native::error!("fetch_and_decode_events error: {:?}", e);
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

pub fn to_payload(
    event: &ethereum_client::LogEvent<ethereum_client::LockEvent>,
) -> anyhow::Result<chains::EthPayload> {
    let block_number: u32 = hex_to_u32(event.block_number.clone())?;
    let log_index: u32 = hex_to_u32(event.log_index.clone())?;

    let event = chains::EthereumEvent::LockEvent {
        id: (block_number, log_index),
    };
    let payload: Vec<u8> = event.encode();
    Ok(payload)
}

fn hex_to_u32(hex_data: String) -> anyhow::Result<u32> {
    let without_prefix = hex_data.trim_start_matches("0x");
    let u32_data = u32::from_str_radix(without_prefix, 16).map_err(|e| {
        debug::native::error!("hex_to_u32 error {:?}", e);
        return anyhow::anyhow!(
            "Error decoding number in hex format {:?}: {:?}",
            without_prefix,
            e
        );
    })?;
    Ok(u32_data)
}
