use crate::{
    chains::{Chain, ChainBlock, ChainBlockNumber, ChainBlocks, ChainId, Ethereum},
    debug, params,
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
) -> Result<ChainBlock, Reason> {
    match chain_id {
        ChainId::Reserved => Err(Reason::Unreachable),
        ChainId::Eth => Ok(fetch_eth_block(number).map(ChainBlock::Eth)?),
        ChainId::Dot => Err(Reason::Unreachable),
    }
}

/// Fetch more blocks from the underlying chain.
pub fn fetch_chain_blocks(
    chain_id: ChainId,
    from: ChainBlockNumber,
    to: ChainBlockNumber,
) -> Result<ChainBlocks, Reason> {
    match chain_id {
        ChainId::Reserved => Err(Reason::Unreachable),
        ChainId::Eth => Ok(fetch_eth_blocks(from, to)?),
        ChainId::Dot => Err(Reason::Unreachable),
    }
}

/// Fetch a single block from the Etherum Starport.
fn fetch_eth_block(number: ChainBlockNumber) -> Result<EthereumBlock, EventError> {
    let eth_rpc_url = runtime_interfaces::validator_config_interface::get_eth_rpc_url()
        .ok_or(EventError::NoRpcUrl)?;
    let eth_block = ethereum_client::get_block(&eth_rpc_url, &params::ETH_STARPORT_ADDRESS, number)
        .map_err(EventError::EthereumClientError)?;
    Ok(eth_block)
}

/// Fetch blocks from the Ethereum Starport, return up to `slack` blocks to add to the event queue.
fn fetch_eth_blocks(
    from: ChainBlockNumber,
    to: ChainBlockNumber,
) -> Result<ChainBlocks, EventError> {
    debug!("Fetching Eth Blocks [{}-{}]", from, to);
    let mut acc: Vec<<Ethereum as Chain>::Block> = vec![];
    for block_number in from..to {
        match fetch_eth_block(block_number) {
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
