use codec::alloc::string::String;
use frame_support::debug;

use sp_std::vec::Vec;

extern crate ethereum_client;

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
        debug::native::error!("fetch_events error: {:?}", e);
        return anyhow::anyhow!("missing 0x prefix");
    })?;

    // Build parameters set for fetching starport `Lock` events
    let fetch_events_request = format!(
        r#"{{"address": "{}", "fromBlock": "{}", "toBlock": "{}", "topics":["{}"]}}"#,
        ETH_STARPORT_ADDRESS, from_block, latest_eth_block, LOCK_EVENT_TOPIC
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

// pub fn to_lock_payload(
//     event: &ethereum_client::LogEvent<ethereum_client::LockEvent>,
// ) -> NoticePayload {
//     let message = encode(notice);
//     // TODO: do signer by chain
//     let signer = "0x6a72a2f14577D9Cd0167801EFDd54a07B40d2b61"
//         .as_bytes()
//         .to_vec();
//     NoticePayload {
//         // id: move id,
//         sig: sign(&message),
//         msg: message.to_vec(),
//         signer: AccountIdent {
//             chain: ChainIdent::Eth,
//             account: signer,
//         },
//     }
// }
