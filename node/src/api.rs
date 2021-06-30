use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::sync::Arc;

use jsonrpc_core::{Error as RpcError, ErrorCode as RpcErrorCode, Result as RpcResult};
use jsonrpc_derive::rpc;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use pallet_cash::{
    chains::{ChainAccount, ChainAsset},
    core::BTreeMap,
    portfolio::Portfolio,
    rates::APR,
    reason::Reason,
    types::{AssetAmount, AssetBalance, AssetInfo, InterestRateModel, Symbol, ValidatorKeys},
};

use pallet_cash_runtime_api::CashApi as CashRuntimeApi;
use pallet_oracle::types::AssetPrice;

use types_derive::{type_alias, Types};

const RUNTIME_ERROR: i64 = 1;
const CHAIN_ERROR: i64 = 2;

// Note: no 128 bit integers for the moment
//  due to issues with serde/serde_json
#[type_alias]
pub type ApiAPR = u64;

#[type_alias]
pub type ApiRates = (ApiAPR, ApiAPR);

#[derive(Deserialize, Serialize, Types)]
pub struct ApiAssetData {
    asset: String,
    balance: String,
    total_supply: String,
    total_borrow: String,
    supply_rate: String,
    borrow_rate: String,
    liquidity_factor: String,
    price: String,
}

#[derive(Deserialize, Serialize, Types)]
pub struct ApiAssetMeta {
    supplier_counts: BTreeMap<String, u32>,
    borrower_counts: BTreeMap<String, u32>,
    combined_supplier_count: u32,
    combined_borrower_count: u32,
}

#[derive(Deserialize, Serialize, Types)]
pub enum ApiInterestRateModel {
    Kink {
        zero_rate: String,
        kink_rate: String,
        kink_utilization: String,
        full_rate: String,
    },
    Fixed {
        rate: String,
    },
}

#[derive(Deserialize, Serialize, Types)]
pub struct ApiAssetInfo {
    asset: ChainAsset,
    decimals: u8,
    liquidity_factor: String,
    rate_model: ApiInterestRateModel,
    miner_shares: String,
    supply_cap: String,
    symbol: Symbol,
    ticker: String,
}

#[derive(Deserialize, Serialize, Types)]
pub struct ApiCashData {
    balance: String,
    cash_yield: String,
    price: String,
    cash_principal: String,
    cash_index: String,
    total_cash: String,
}

#[derive(Deserialize, Serialize, Types)]
pub struct ApiPortfolio {
    cash: String,
    positions: Vec<(ChainAsset, String)>,
}

#[derive(Deserialize, Serialize, Types)]
pub struct ApiValidators {
    current_block: String,
    current_validators: Vec<(String, String)>,
    miner_payouts: Vec<(ChainAccount, String)>,
}

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
pub trait GatewayRpcApi<BlockHash> {
    #[rpc(name = "gateway_assetdata")]
    fn gateway_assetdata(
        &self,
        account: ChainAccount,
        assets: ChainAsset,
        at: Option<BlockHash>,
    ) -> RpcResult<ApiAssetData>;

    #[rpc(name = "gateway_assetmeta")]
    fn chain_assets_meta(&self, at: Option<BlockHash>) -> RpcResult<ApiAssetMeta>;

    #[rpc(name = "gateway_cashdata")]
    fn gateway_cashdata(
        &self,
        account: ChainAccount,
        at: Option<BlockHash>,
    ) -> RpcResult<ApiCashData>;

    #[rpc(name = "gateway_liquidity")]
    fn gateway_liquidity(&self, account: ChainAccount, at: Option<BlockHash>) -> RpcResult<String>;

    #[rpc(name = "gateway_price")]
    fn gateway_price(&self, ticker: String, at: Option<BlockHash>) -> RpcResult<String>;

    #[rpc(name = "gateway_rates")]
    fn gateway_rates(&self, asset: ChainAsset, at: Option<BlockHash>) -> RpcResult<ApiRates>;

    #[rpc(name = "gateway_assets")]
    fn gateway_assets(&self, at: Option<BlockHash>) -> RpcResult<Vec<ApiAssetInfo>>;

    #[rpc(name = "gateway_chain_accounts")]
    fn chain_accounts(&self, at: Option<BlockHash>) -> RpcResult<Vec<ChainAccount>>;

    #[rpc(name = "gateway_chain_account_liquidities")]
    fn chain_account_liquidities(
        &self,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<(ChainAccount, String)>>;

    #[rpc(name = "gateway_chain_account_portfolio")]
    fn chain_account_portfolio(
        &self,
        account: ChainAccount,
        at: Option<BlockHash>,
    ) -> RpcResult<ApiPortfolio>;

    #[rpc(name = "gateway_validators")]
    fn validators(&self, at: Option<BlockHash>) -> RpcResult<ApiValidators>;
}

pub struct GatewayRpcHandler<C, B> {
    client: Arc<C>,
    _block: PhantomData<B>,
}

impl<C, B> GatewayRpcHandler<C, B> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _block: Default::default(),
        }
    }
}

impl<C, B> GatewayRpcApi<B::Hash> for GatewayRpcHandler<C, B>
where
    B: BlockT,
    C: 'static + Send + Sync + ProvideRuntimeApi<B> + HeaderBackend<B>,
    C::Api: CashRuntimeApi<B>,
{
    fn gateway_assetdata(
        &self,
        account: ChainAccount,
        asset: ChainAsset,
        at: Option<<B as BlockT>::Hash>,
    ) -> RpcResult<ApiAssetData> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let asset_info: AssetInfo = api
            .get_asset(&at, asset)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        let account_balance: AssetBalance = api
            .get_account_balance(&at, account, asset)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        let (total_borrow, total_supply): (AssetAmount, AssetAmount) = api
            .get_market_totals(&at, asset)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        let (borrow_rate, supply_rate): (APR, APR) = api
            .get_rates(&at, asset)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        let price: AssetPrice = api
            .get_price_with_ticker(&at, asset_info.ticker)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        Ok(ApiAssetData {
            asset: asset.into(),
            balance: format!("{}", account_balance),
            total_supply: format!("{}", total_supply),
            total_borrow: format!("{}", total_borrow),
            supply_rate: format!("{}", supply_rate.0),
            borrow_rate: format!("{}", borrow_rate.0),
            liquidity_factor: format!("{}", asset_info.liquidity_factor.0),
            price: format!("{}", price),
        })
    }

    fn gateway_cashdata(
        &self,
        account: ChainAccount,
        at: Option<<B as BlockT>::Hash>,
    ) -> RpcResult<ApiCashData> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let balance: AssetBalance = api
            .get_full_cash_balance(&at, account)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        let cash_yield: APR = api
            .get_cash_yield(&at)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        let price: AssetPrice = api
            .get_price(&at, "CASH".to_string())
            .map_err(runtime_err)?
            .map_err(chain_err)?;
        let (cash_index, total_principal, total_cash) = api
            .get_cash_data(&at)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        Ok(ApiCashData {
            balance: format!("{}", balance),
            cash_yield: format!("{}", cash_yield.0),
            price: format!("{}", price),
            cash_principal: format!("{}", total_principal.0),
            cash_index: format!("{}", cash_index.0),
            total_cash: format!("{}", total_cash.value),
        })
    }

    fn gateway_liquidity(
        &self,
        account: ChainAccount,
        at: Option<<B as BlockT>::Hash>,
    ) -> RpcResult<String> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let result: AssetBalance = api
            .get_liquidity(&at, account)
            .map_err(runtime_err)?
            .map_err(chain_err)?;
        Ok(format!("{}", result))
    }

    fn gateway_price(&self, ticker: String, at: Option<<B as BlockT>::Hash>) -> RpcResult<String> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let result: AssetPrice = api
            .get_price(&at, ticker)
            .map_err(runtime_err)?
            .map_err(chain_err)?;
        Ok(format!("{}", result))
    }

    fn gateway_rates(
        &self,
        asset: ChainAsset,
        at: Option<<B as BlockT>::Hash>,
    ) -> RpcResult<ApiRates> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let (borrow_rate, supply_rate): (APR, APR) = api
            .get_rates(&at, asset)
            .map_err(runtime_err)?
            .map_err(chain_err)?;
        Ok((borrow_rate.0 as ApiAPR, supply_rate.0 as ApiAPR)) // XXX try_into?
    }

    fn gateway_assets(&self, at: Option<<B as BlockT>::Hash>) -> RpcResult<Vec<ApiAssetInfo>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let assets = api
            .get_assets(&at)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        fn api_rate_model(model: InterestRateModel) -> ApiInterestRateModel {
            match model {
                InterestRateModel::Kink {
                    zero_rate,
                    kink_rate,
                    kink_utilization,
                    full_rate,
                } => ApiInterestRateModel::Kink {
                    zero_rate: format!("{:?}", zero_rate.0),
                    kink_rate: format!("{:?}", kink_rate.0),
                    kink_utilization: format!("{:?}", kink_utilization.0),
                    full_rate: format!("{:?}", full_rate.0),
                },
                InterestRateModel::Fixed { rate } => ApiInterestRateModel::Fixed {
                    rate: String::from(rate),
                },
            }
        }

        let assets_lite = assets
            .iter()
            .map(|asset_info| ApiAssetInfo {
                asset: asset_info.asset,
                decimals: asset_info.decimals,
                liquidity_factor: format!("{}", asset_info.liquidity_factor.0),
                rate_model: api_rate_model(asset_info.rate_model),
                miner_shares: format!("{}", asset_info.miner_shares.0),
                supply_cap: format!("{}", asset_info.supply_cap),
                symbol: asset_info.symbol,
                ticker: String::from(asset_info.ticker),
            })
            .collect();

        Ok(assets_lite) // XXX try_into?
    }

    fn chain_accounts(&self, at: Option<<B as BlockT>::Hash>) -> RpcResult<Vec<ChainAccount>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let accounts = api
            .get_accounts(&at)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        Ok(accounts)
    }

    fn chain_assets_meta(&self, at: Option<<B as BlockT>::Hash>) -> RpcResult<ApiAssetMeta> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let (supplier_counts, borrower_counts, combined_suppliers, combined_borrowers) = api
            .get_asset_meta(&at)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        Ok(ApiAssetMeta {
            supplier_counts: supplier_counts,
            borrower_counts: borrower_counts,
            combined_supplier_count: combined_suppliers,
            combined_borrower_count: combined_borrowers,
        })
    }

    fn chain_account_liquidities(
        &self,
        at: Option<<B as BlockT>::Hash>,
    ) -> RpcResult<Vec<(ChainAccount, String)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let accounts = api
            .get_accounts_liquidity(&at)
            .map_err(runtime_err)?
            .map_err(chain_err)?;
        Ok(accounts) // XXX try_into?
    }

    fn chain_account_portfolio(
        &self,
        account: ChainAccount,
        at: Option<<B as BlockT>::Hash>,
    ) -> RpcResult<ApiPortfolio> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let result: Portfolio = api
            .get_portfolio(&at, account)
            .map_err(runtime_err)?
            .map_err(chain_err)?;

        let positions_lite = result
            .positions
            .iter()
            .map(|p| (p.0.asset, format!("{}", p.1.value)))
            .collect();
        print!("{:?}", positions_lite);
        Ok(ApiPortfolio {
            cash: format!("{}", result.cash.value),
            positions: positions_lite,
        })
    }

    fn validators(&self, at: Option<<B as BlockT>::Hash>) -> RpcResult<ApiValidators> {
        let api = self.client.runtime_api();
        let hash = at.unwrap_or_else(|| self.client.info().best_hash);
        let at = BlockId::hash(hash.clone());
        let current_block = self
            .client
            .number(hash)
            .unwrap_or_else(|_| None)
            .unwrap_or_else(|| self.client.info().best_number);

        let (validator_keys, miner_payouts): (Vec<ValidatorKeys>, Vec<(ChainAccount, String)>) =
            api.get_validator_info(&at)
                .map_err(runtime_err)?
                .map_err(chain_err)?;

        let readable_validator_keys = validator_keys
            .iter()
            .map(|v| (format!("{}", v.substrate_id), hex::encode(v.eth_address)))
            .collect();

        Ok(ApiValidators {
            current_block: current_block.to_string(),
            current_validators: readable_validator_keys,
            miner_payouts: miner_payouts,
        })
    }
}
