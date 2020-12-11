use codec::alloc::string::String;
use frame_support::debug;

use sp_std::vec::Vec;

extern crate ethereum_client;

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
    let eth_rpc_url = String::from_utf8(config.get_eth_rpc_url())
        .map_err(|e| return anyhow::anyhow!("Error reading `eth_rpc_url` from config {:?}", e))?;
    let eth_starport_address =
        String::from_utf8(config.get_eth_starport_address()).map_err(|e| {
            return anyhow::anyhow!("Error reading `eth_starport_address` from config {:?}", e);
        })?;
    let eth_lock_event_topic =
        String::from_utf8(config.get_eth_lock_event_topic()).map_err(|e| {
            return anyhow::anyhow!("Error reading `eth_lock_event_topic` from config {:?}", e);
        })?;

    // Fetch the latest available ethereum block number
    let latest_eth_block = ethereum_client::fetch_latest_block(&eth_rpc_url).map_err(|e| {
        debug::native::error!("fetch_events error: {:?}", e);
        return anyhow::anyhow!("missing 0x prefix");
    })?;

    // Build parameters set for fetching starport `Lock` events
    let fetch_events_request = format!(
        r#"{{"address": "{}", "fromBlock": "{}", "toBlock": "{}", "topics":["{}"]}}"#,
        eth_starport_address, from_block, latest_eth_block, eth_lock_event_topic
    );

    // Fetch `Lock` events using ethereum_client
    let lock_events =
        ethereum_client::fetch_and_decode_events(&eth_rpc_url, vec![&fetch_events_request])
            .map_err(|e| return anyhow::anyhow!("missing 0x prefix"))?;

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
