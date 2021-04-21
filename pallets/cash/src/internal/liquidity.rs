// Note: The substrate build requires these be re-exported.
pub use our_std::{
    cmp::{max, min},
    collections::btree_set::BTreeSet,
    convert::{TryFrom, TryInto},
    fmt, if_std, result,
    result::Result,
    str,
};

use crate::{
    chains::ChainAccount,
    portfolio::Portfolio,
    reason::Reason,
    types::{AssetInfo, AssetQuantity, Balance, CashQuantity},
    Config,
};

// Liquidity Checks //

/// Calculates if an account will remain solvent after reducing asset by amount.
pub fn has_liquidity_to_reduce_asset<T: Config>(
    account: ChainAccount,
    asset: AssetInfo,
    amount: AssetQuantity,
) -> Result<bool, Reason> {
    if asset.ticker != amount.units.ticker {
        Err(Reason::AssetQuantityMismatch)?
    }
    let liquidity = Portfolio::from_storage::<T>(account)?
        .asset_change(asset, amount.as_decrease()?)?
        .get_liquidity::<T>()?;
    Ok(liquidity.value >= 0)
}

/// Calculates if an account will remain solvent after reducing CASH by amount.
pub fn has_liquidity_to_reduce_cash<T: Config>(
    account: ChainAccount,
    amount: CashQuantity,
) -> Result<bool, Reason> {
    let portfolio = Portfolio::from_storage::<T>(account)?;
    let liquidity = portfolio
        .cash_change(amount.as_decrease()?)?
        .get_liquidity::<T>()?;
    Ok(liquidity.value >= 0)
}

/// Calculates the current liquidity value for an account.
pub fn get_liquidity<T: Config>(account: ChainAccount) -> Result<Balance, Reason> {
    Ok(Portfolio::from_storage::<T>(account)?.get_liquidity::<T>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::*,
        tests::{assets::*, common::*, mock::*},
        types::*,
        SupportedAssets,
    };
    use frame_support::storage::StorageMap;
    use pallet_oracle::{self, types::Price};

    #[test]
    fn test_has_liquidity_to_reduce_asset() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("3", ETH).value);

            vec![
                (eth, Quantity::from_nominal("0", ETH), true),
                (eth, Quantity::from_nominal("1", ETH), true),
                (eth, Quantity::from_nominal("3", ETH), true),
                (eth, Quantity::from_nominal("4", ETH), false),
                (wbtc, Quantity::from_nominal("0.01", WBTC), true),
                (wbtc, Quantity::from_nominal("0.1", WBTC), false),
            ]
            .into_iter()
            .for_each(|(asset, amount, expected)| {
                let given = has_liquidity_to_reduce_asset::<Test>(account, asset, amount);
                if given != Ok(expected) {
                    panic!("has_liquidity_to_reduce_asset(account, {}, {:?}): Expected={:?}, Given={:?}", String::from(asset.symbol), amount, expected, given);
                }
            })
        })
    }

    #[test]
    fn test_has_liquidity_to_reduce_asset_mismatch() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                has_liquidity_to_reduce_asset::<Test>(
                    ChainAccount::Eth([0u8; 20]),
                    eth,
                    Quantity::from_nominal("0.01", WBTC)
                ),
                Err(Reason::AssetQuantityMismatch)
            );
        })
    }

    #[test]
    fn test_has_liquidity_to_reduce_asset_unpriced() {
        new_test_ext().execute_with(|| {
            pallet_oracle::Prices::insert(
                ETH.ticker,
                Price::from_nominal(ETH.ticker, "2000.00").value,
            );
            SupportedAssets::insert(&Eth, eth);

            assert_eq!(
                has_liquidity_to_reduce_asset::<Test>(
                    ChainAccount::Eth([0u8; 20]),
                    wbtc,
                    Quantity::from_nominal("0.01", WBTC)
                ),
                Err(Reason::NoPrice)
            );
        })
    }

    #[test]
    fn test_has_liquidity_to_reduce_cash() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("1", ETH).value); // 2000 * 0.8 = 1600
            init_asset_balance(Wbtc, account, Balance::from_nominal("0.1", WBTC).value); // 60000 * 0.6 = 3600
                                                                                         // Total Liquidity = 1600 + 3600 = 5200

            vec![
                (Quantity::from_nominal("0", CASH), true),
                (Quantity::from_nominal("100", CASH), true),
                (Quantity::from_nominal("5000", CASH), true),
                (Quantity::from_nominal("5200", CASH), true),
                (Quantity::from_nominal("5300", CASH), false),
                (Quantity::from_nominal("10000", CASH), false),
            ]
            .into_iter()
            .for_each(|(cash_amount, expected)| {
                let given = has_liquidity_to_reduce_cash::<Test>(account, cash_amount);
                if given != Ok(expected) {
                    panic!(
                        "has_liquidity_to_reduce_asset(account, {:?}): Expected={:?}, Given={:?}",
                        cash_amount, expected, given
                    );
                }
            })
        })
    }
}
