use pallet_cash::{
    chains::{ChainAccount, ChainAsset},
    rates::APR,
    reason::Reason,
    types::{AssetBalance, AssetPrice},
};

sp_api::decl_runtime_apis! {
    pub trait CashApi {
        fn get_liquidity(account: ChainAccount) -> Result<AssetBalance, Reason>;
        fn get_price(ticker: String) -> Result<AssetPrice, Reason>;
        fn get_rates(asset: ChainAsset) -> Result<(APR, APR), Reason>;
    }
}
