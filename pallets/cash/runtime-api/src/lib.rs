use pallet_cash::{
    chains::{ChainAccount, ChainAsset},
    portfolio::Portfolio,
    rates::APR,
    reason::Reason,
    types::{AssetAmount, AssetBalance, AssetInfo},
};
use pallet_oracle::{ticker::Ticker, types::AssetPrice};

sp_api::decl_runtime_apis! {
    pub trait CashApi {
        fn get_account_balance(account: ChainAccount, asset: ChainAsset) -> Result<AssetBalance, Reason>;
        fn get_asset(asset: ChainAsset) -> Result<AssetInfo, Reason>;
        fn get_cash_yield() -> Result<APR, Reason>;
        fn get_full_cash_balance(account: ChainAccount) -> Result<AssetBalance, Reason>;
        fn get_liquidity(account: ChainAccount) -> Result<AssetBalance, Reason>;
        fn get_market_totals(asset: ChainAsset) -> Result<(AssetAmount, AssetAmount), Reason>;
        fn get_price(ticker: String) -> Result<AssetPrice, Reason>;
        fn get_price_with_ticker(ticker: Ticker) -> Result<AssetPrice, Reason>;
        fn get_rates(asset: ChainAsset) -> Result<(APR, APR), Reason>;
        fn get_accounts() -> Result<Vec<ChainAccount>, Reason>;
        fn get_accounts_liquidity() -> Result<Vec<(ChainAccount, String)>, Reason>;
        fn get_portfolio(account: ChainAccount) -> Result<Portfolio, Reason>;
    }
}
