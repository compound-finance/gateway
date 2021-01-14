use crate::chains;
use codec::alloc::string::String;
use codec::Encode;
use frame_support::debug;
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

    debug::native::print!(
        "eth_rpc_url={}, starport_address={}, lock_event_topic={}",
        eth_rpc_url,
        eth_starport_address,
        eth_lock_event_topic
    );

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

pub fn get_next_block_hex(
    saved_block_hex: String,
    pending_events_block: u32,
) -> anyhow::Result<String> {
    let saved_block = hex_to_u32(saved_block_hex)?;
    let res_block_num = if saved_block > pending_events_block {
        saved_block
    } else {
        pending_events_block
    };
    let next_block_num_hex = format!("{:#X}", res_block_num + 1);
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
        debug::native::error!("hex_to_u32 error {:?}", e);
        return anyhow::anyhow!(
            "Error decoding number in hex format {:?}: {:?}",
            without_prefix,
            e
        );
    })?;
    Ok(u32_data)
}
