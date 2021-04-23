use crate::{
    chains::{ChainBlock, ChainBlockNumber, ChainBlocks, ChainId},
    reason::Reason,
};
use codec::{Decode, Encode};
use ethereum_client::EthereumClientError;
use our_std::RuntimeDebug;
use types_derive::Types;

/// Type for errors coming from event ingression.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum EventError {
    NoRpcUrl,
    NoStarportAddress,
    EthereumClientError(EthereumClientError),
    ErrorDecodingHex,
    NoDeadline,
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
    let eth_starport_address = runtime_interfaces::config_interface::get_eth_starport_address()
        .ok_or(EventError::NoStarportAddress)?;
    let eth_rpc_url = runtime_interfaces::validator_config_interface::get_eth_rpc_url()
        .ok_or(EventError::NoRpcUrl)?;
    let eth_fetch_deadline =
        runtime_interfaces::validator_config_interface::get_eth_fetch_deadline()
            .ok_or(EventError::NoRpcUrl)?;
    let eth_chain_block = ethereum_client::get_block(
        &eth_rpc_url,
        &eth_starport_address,
        number,
        eth_fetch_deadline,
    )
    .map_err(EventError::EthereumClientError)?;
    Ok(ChainBlock::Eth(eth_chain_block))
}

/// Fetch blocks from the Ethereum Starport, return up to `slack` blocks to add to the event queue.
fn fetch_eth_blocks(number: ChainBlockNumber, slack: u32) -> Result<ChainBlocks, Reason> {
    match slack {
        0 => Ok(ChainBlocks::Eth(vec![])),
        _ => {
            // Note: can be optimized to return up to `slack` blocks, for now just one
            Ok(fetch_eth_block(number)?.into())
        }
    }
}
