use crate::{symbol::USD, tests::*};

pub const ETH: Units = Units::from_ticker_str("ETH", 18);
pub const Eth: ChainAsset = ChainAsset::Eth(hex!("EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"));
pub const eth: AssetInfo = AssetInfo {
    asset: Eth,
    decimals: ETH.decimals,
    liquidity_factor: LiquidityFactor::from_nominal("0.8"),
    rate_model: InterestRateModel::Kink {
        zero_rate: APR(0),
        kink_rate: APR(200),
        kink_utilization: Factor::from_nominal("0.9"),
        full_rate: APR(5000),
    },
    miner_shares: Factor::from_nominal("0.05"),
    supply_cap: Quantity::from_nominal("1000", ETH).value,
    symbol: Symbol(ETH.ticker.0),
    ticker: Ticker(ETH.ticker.0),
};

pub const UNI: Units = Units::from_ticker_str("UNI", 18);
pub const Uni: ChainAsset = ChainAsset::Eth(hex!("1f9840a85d5af5bf1d1762f925bdaddc4201f984"));
pub const uni: AssetInfo = AssetInfo {
    asset: Uni,
    decimals: UNI.decimals,
    liquidity_factor: LiquidityFactor::from_nominal("0.7"),
    rate_model: InterestRateModel::Kink {
        zero_rate: APR(0),
        kink_rate: APR(500),
        kink_utilization: Factor::from_nominal("0.8"),
        full_rate: APR(2000),
    },
    miner_shares: Factor::from_nominal("0.05"),
    supply_cap: Quantity::from_nominal("1000", UNI).value,
    symbol: Symbol(UNI.ticker.0),
    ticker: Ticker(UNI.ticker.0),
};

pub const WBTC: Units = Units::from_ticker_str("WBTC", 8);
pub const Wbtc: ChainAsset = ChainAsset::Eth(hex!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
pub const wbtc: AssetInfo = AssetInfo {
    asset: Wbtc,
    decimals: WBTC.decimals,
    liquidity_factor: LiquidityFactor::from_nominal("0.6"),
    rate_model: InterestRateModel::Kink {
        zero_rate: APR(0),
        kink_rate: APR(500),
        kink_utilization: Factor::from_nominal("0.8"),
        full_rate: APR(2000),
    },
    miner_shares: Factor::from_nominal("0.05"),
    supply_cap: Quantity::from_nominal("1000", WBTC).value,
    symbol: Symbol(WBTC.ticker.0),
    ticker: Ticker(WBTC.ticker.0),
};

pub const Usdc: ChainAsset = ChainAsset::Eth(hex!("cccccccccccccccccccccccccccccccccccccccc"));
pub const usdc: AssetInfo = AssetInfo {
    asset: Usdc,
    decimals: USD.decimals,
    liquidity_factor: LiquidityFactor::from_nominal("0.9"),
    rate_model: InterestRateModel::Kink {
        zero_rate: APR(0),
        kink_rate: APR(500),
        kink_utilization: Factor::from_nominal("0.8"),
        full_rate: APR(2000),
    },
    miner_shares: Factor::from_nominal("0.05"),
    supply_cap: Quantity::from_nominal("1000", USD).value,
    symbol: Symbol(USD.ticker.0),
    ticker: Ticker(USD.ticker.0),
};
