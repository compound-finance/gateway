use std::marker::PhantomData;
use std::sync::Arc;

use jsonrpc_core::{Error as RpcError, ErrorCode as RpcErrorCode, Result as RpcResult};
use jsonrpc_derive::rpc;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use pallet_cash::{
    chains::ChainAccount,
    reason::Reason,
    types::{AssetBalance, AssetPrice},
};
use pallet_cash_runtime_api::CashApi as CashRuntimeApi;

const RUNTIME_ERROR: i64 = 1;
const CHAIN_ERROR: i64 = 2;

pub type ApiAssetBalance = i64; // XXX ugh no i128s for the moment
pub type ApiAssetPrice = u64; // XXX ugh no u128s for the moment

/// Converts a runtime trap into an RPC error.
fn runtime_err(err: impl std::fmt::Debug) -> RpcError {
    RpcError {
        code: RpcErrorCode::ServerError(RUNTIME_ERROR),
        message: "Runtime trapped".into(),
        data: Some(format!("{:?}", err).into()),
    }
}

/// Converts a chain failure into an RPC error.
fn chain_err(reason: Reason) -> RpcError {
    RpcError {
        code: RpcErrorCode::ServerError(CHAIN_ERROR),
        message: "Chain error".into(),
        data: Some(format!("{:?}", reason).into()),
    }
}

#[rpc]
pub trait CompoundRpcApi<BlockHash> {
    #[rpc(name = "get_liquidity")]
    fn get_liquidity(
        &self,
        account: ChainAccount,
        at: Option<BlockHash>,
    ) -> RpcResult<ApiAssetBalance>;

    #[rpc(name = "get_price")]
    fn get_price(&self, ticker: String, at: Option<BlockHash>) -> RpcResult<ApiAssetPrice>;
}

pub struct CompoundRpcHandler<C, B> {
    client: Arc<C>,
    _block: PhantomData<B>,
}

impl<C, B> CompoundRpcHandler<C, B> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _block: Default::default(),
        }
    }
}

impl<C, B> CompoundRpcApi<B::Hash> for CompoundRpcHandler<C, B>
where
    B: BlockT,
    C: 'static + Send + Sync + ProvideRuntimeApi<B> + HeaderBackend<B>,
    C::Api: CashRuntimeApi<B>,
{
    fn get_liquidity(
        &self,
        account: ChainAccount,
        at: Option<<B as BlockT>::Hash>,
    ) -> RpcResult<ApiAssetBalance> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let result: AssetBalance = api
            .get_liquidity(&at, account)
            .map_err(runtime_err)?
            .map_err(chain_err)?;
        Ok((result as ApiAssetBalance).into()) // XXX ugh
    }

    fn get_price(
        &self,
        ticker: String,
        at: Option<<B as BlockT>::Hash>,
    ) -> RpcResult<ApiAssetPrice> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let result: AssetPrice = api
            .get_price(&at, ticker)
            .map_err(runtime_err)?
            .map_err(chain_err)?;
        Ok((result as ApiAssetPrice).into()) // XXX ugh
    }
}
