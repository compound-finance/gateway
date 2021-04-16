use codec::{alloc::string::String, Decode, Encode};

use ethereum_client::EthereumClientError;
use our_std::{vec::Vec, RuntimeDebug};
use types_derive::Types;

use crate::{
    chains::{
        Chain, ChainBlock, ChainBlockNumber, ChainBlockTally, ChainBlocks, ChainId, ChainSignature,
        Ethereum,
    },
    log,
    reason::Reason,
    types::SignersSet,
};

/// Type for errors coming from event ingression.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EventError {
    NoRpcUrl,
    BadRpcUrl,
    BadStarportAddress,
    EthereumClientError(EthereumClientError),
    ErrorDecodingHex,
}

/// Fetch a block from the underlying chain.
pub fn fetch_chain_block(
    chain_id: ChainId,
    number: ChainBlockNumber,
) -> Result<ChainBlock, Reason> {
    match chain_id {
        ChainId::Eth => fetch_eth_block(number),
        ChainId::Dot => Err(Reason::Unreachable),
    }
}

/// Fetch more blocks from the underlying chain.
pub fn fetch_chain_blocks(
    chain_id: ChainId,
    number: ChainBlockNumber,
    nblocks: u32,
) -> Result<ChainBlocks, Reason> {
    match chain_id {
        ChainId::Eth => fetch_eth_blocks(number, nblocks),
        ChainId::Dot => Err(Reason::Unreachable),
    }
}

/// Fetch a single block from the Etherum Starport.
fn fetch_eth_block(number: ChainBlockNumber) -> Result<ChainBlock, Reason> {
    let config = runtime_interfaces::config_interface::get();
    let eth_rpc_url = runtime_interfaces::validator_config_interface::get_eth_rpc_url()
        .ok_or(EventError::NoRpcUrl)?;
    let eth_rpc_url = String::from_utf8(eth_rpc_url).map_err(|_| EventError::BadRpcUrl)?;
    let eth_starport_address = String::from_utf8(config.get_eth_starport_address())
        .map_err(|_| EventError::BadStarportAddress)?;
    let eth_chain_block = ethereum_client::get_block(&eth_rpc_url, &eth_starport_address, number)
        .map_err(EventError::EthereumClientError)?;
    Ok(ChainBlock::Eth(eth_chain_block))
}

/// Fetch blocks from the Ethereum Starport, return up to `slack` blocks to add to the event queue.
fn fetch_eth_blocks(number: ChainBlockNumber, slack: u32) -> Result<ChainBlocks, Reason> {
    match slack {
        0 => Ok(ChainBlocks::Eth(vec![])),
        _ => {
            // Note: can be optimized to return up to `slack` blocks, for now just one
            Ok(fetch_eth_block(number)?.into()) // XXX
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{tests::*, *};
    use our_std::convert::*;
    use sp_core::offchain::testing;

    pub fn get_mock_http_calls(events_response: Vec<u8>) -> Vec<testing::PendingRequest> {
        // Set up config values
        let given_eth_starport_address: Vec<u8> =
            "0xbbde1662bC3ED16aA8C618c9833c801F3543B587".into();
        let config = runtime_interfaces::new_config(given_eth_starport_address.clone());
        runtime_interfaces::config_interface::set(config);

        let given_eth_rpc_url =
            runtime_interfaces::validator_config_interface::get_eth_rpc_url().unwrap();
        return vec![
            testing::PendingRequest{
                method: "POST".into(),
                uri: String::from_utf8(given_eth_rpc_url.clone()).unwrap(),
                body: br#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#.to_vec(),
                response: Some(tests::testdata::json_responses::BLOCK_NUMBER_RESPONSE.to_vec()),
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
        // let calls: Vec<testing::PendingRequest> =
        //     get_mock_http_calls(tests::testdata::json_responses::EVENTS_RESPONSE.to_vec());

        // let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);
        // t.execute_with(|| {
        // XXX
        // let events_candidate = events::fetch_eth_blocks("earliest".to_string());
        // assert!(events_candidate.is_ok());
        // let starport_info = events_candidate.unwrap();
        // let latest_eth_block = starport_info.latest_eth_block;
        // let mut events = starport_info.events;
        // events.reverse(); // Since we'll be popping off the end

        // assert_eq!(latest_eth_block, 11695207);
        // assert_eq!(events.len(), 3);
        // if let Some((_chain_log_id, ChainLogEvent::Eth(log))) = events.pop() {
        //     assert_eq!(
        //         Ok(log.block_hash),
        //         hex::decode("c1c0eb37b56923ad9e20fdb31ca882988d5217f7ca24b6297ca6ed700811cf23")
        //             .unwrap()
        //             .try_into()
        //     );
        // } else {
        //     assert!(false);
        // }

        // if let Some((_chain_log_id, ChainLogEvent::Eth(log))) = events.pop() {
        //     assert_eq!(
        //         Ok(log.block_hash),
        //         hex::decode("a5c8024e699a5c30eb965e47b5157c06c76f3b726bff377a0a5333a561f25648")
        //             .unwrap()
        //             .try_into()
        //     );
        // } else {
        //     assert!(false);
        // }

        // if let Some((_chain_log_id, ChainLogEvent::Eth(log))) = events.pop() {
        //     assert_eq!(
        //         Ok(log.block_hash),
        //         hex::decode("a4a96e957718e3a30b77a667f93978d8f438bdcd56ff03545f08c833d9a26687")
        //             .unwrap()
        //             .try_into()
        //     );
        // } else {
        //     assert!(false);
        // }
        // });
    }

    #[test]
    fn test_fetch_events_with_no_events() {
        // let calls: Vec<testing::PendingRequest> =
        //     get_mock_http_calls(tests::testdata::json_responses::NO_EVENTS_RESPONSE.to_vec());

        // let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);
        // t.execute_with(|| {
        // XXX
        // let events_candidate = events::fetch_eth_blocks("earliest".to_string());
        // assert!(events_candidate.is_ok());
        // let event_info = events_candidate.unwrap();
        // let latest_eth_block = event_info.latest_eth_block;

        // assert_eq!(latest_eth_block, 11695207);
        // assert_eq!(event_info.events.len(), 0);
        // });
    }
}
