use crate::{
    chains::{Chain, ChainAccount, ChainBlock, ChainBlockNumber, ChainBlocks, ChainId, Ethereum},
    debug,
    reason::Reason,
};
use codec::{Decode, Encode};
use ethereum_client::{EthereumBlock, EthereumClientError};
use our_std::RuntimeDebug;
use types_derive::Types;

/// Type for errors coming from event ingression.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum EventError {
    NoRpcUrl,
    NoStarportAddress,
    EthereumClientError(EthereumClientError),
    ErrorDecodingHex,
}

/// Fetch a block from the underlying chain.
pub fn fetch_chain_block(
    chain_id: ChainId,
    number: ChainBlockNumber,
    starport: ChainAccount,
) -> Result<ChainBlock, Reason> {
    match (chain_id, starport) {
        (ChainId::Gate, _) => Err(Reason::Unreachable),
        (ChainId::Eth, ChainAccount::Eth(eth_starport_address)) => {
            Ok(fetch_eth_block(number, &eth_starport_address).map(ChainBlock::Eth)?)
        }
        (ChainId::Dot, _) => Err(Reason::Unreachable),
        _ => Err(Reason::Unreachable),
    }
}

/// Fetch more blocks from the underlying chain.
pub fn fetch_chain_blocks(
    chain_id: ChainId,
    from: ChainBlockNumber,
    to: ChainBlockNumber,
    starport: ChainAccount,
) -> Result<ChainBlocks, Reason> {
    match (chain_id, starport) {
        (ChainId::Gate, _) => Err(Reason::Unreachable),
        (ChainId::Eth, ChainAccount::Eth(eth_starport_address)) => {
            Ok(fetch_eth_blocks(from, to, &eth_starport_address)?)
        }
        (ChainId::Dot, _) => Err(Reason::Unreachable),
        _ => Err(Reason::Unreachable),
    }
}

/// Fetch a single block from the Etherum Starport.
fn fetch_eth_block(
    number: ChainBlockNumber,
    eth_starport_address: &[u8; 20],
) -> Result<EthereumBlock, EventError> {
    let eth_rpc_url = runtime_interfaces::validator_config_interface::get_eth_rpc_url()
        .ok_or(EventError::NoRpcUrl)?;
    let eth_block = ethereum_client::get_block(&eth_rpc_url, eth_starport_address, number)
        .map_err(EventError::EthereumClientError)?;
    Ok(eth_block)
}

/// Fetch blocks from the Ethereum Starport, return up to `slack` blocks to add to the event queue.
fn fetch_eth_blocks(
    from: ChainBlockNumber,
    to: ChainBlockNumber,
    eth_starport_address: &[u8; 20],
) -> Result<ChainBlocks, EventError> {
    debug!("Fetching Eth Blocks [{}-{}]", from, to);
    let mut acc: Vec<<Ethereum as Chain>::Block> = vec![];
    for block_number in from..to {
        match fetch_eth_block(block_number, eth_starport_address) {
            Ok(block) => {
                acc.push(block);
            }
            Err(EventError::EthereumClientError(EthereumClientError::NoResult)) => {
                break; // done
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
    Ok(ChainBlocks::Eth(acc))
}
