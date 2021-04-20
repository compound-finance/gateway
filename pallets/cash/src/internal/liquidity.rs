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

/// Calculates if an account has non-negative liquidity
pub fn has_non_negative_liquidity<T: Config>(account: ChainAccount) -> Result<bool, Reason> {
    let liquidity = Portfolio::from_storage::<T>(account)?.get_liquidity::<T>()?;
    Ok(liquidity.value >= 0)
}

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

/// Calculates if an account will remain solvent after reducing asset by amount and paying a CASH fee.
pub fn has_liquidity_to_reduce_asset_with_fee<T: Config>(
    account: ChainAccount,
    asset: AssetInfo,
    amount: AssetQuantity,
    fee: CashQuantity,
) -> Result<bool, Reason> {
    if asset.ticker != amount.units.ticker {
        Err(Reason::AssetQuantityMismatch)?
    }
    let liquidity = Portfolio::from_storage::<T>(account)?
        .asset_change(asset, amount.as_decrease()?)?
        .cash_change(fee.as_decrease()?)?
        .get_liquidity::<T>()?;
    Ok(liquidity.value >= 0)
}

/// Calculates if an account will remain solvent after reducing asset by amount and adding an amount of asset collateral.
pub fn has_liquidity_to_reduce_asset_with_added_collateral<T: Config>(
    account: ChainAccount,
    asset: AssetInfo,
    amount: AssetQuantity,
    collateral_asset: AssetInfo,
    collateral_amount: AssetQuantity,
) -> Result<bool, Reason> {
    if asset.ticker != amount.units.ticker {
        Err(Reason::AssetQuantityMismatch)?
    }
    if collateral_asset.ticker != collateral_amount.units.ticker {
        Err(Reason::AssetQuantityMismatch)?
    }
    let liquidity = Portfolio::from_storage::<T>(account)?
        .asset_change(asset, amount.as_decrease()?)?
        .asset_change(collateral_asset, collateral_amount.as_increase()?)?
        .get_liquidity::<T>()?;
    Ok(liquidity.value >= 0)
}

/// Calculates if an account will remain solvent after reducing asset by amount and adding an amount of CASH collateral.
pub fn has_liquidity_to_reduce_asset_with_added_cash<T: Config>(
    account: ChainAccount,
    asset: AssetInfo,
    amount: AssetQuantity,
    cash_amount: CashQuantity,
) -> Result<bool, Reason> {
    if asset.ticker != amount.units.ticker {
        Err(Reason::AssetQuantityMismatch)?
    }
    let liquidity = Portfolio::from_storage::<T>(account)?
        .asset_change(asset, amount.as_decrease()?)?
        .cash_change(cash_amount.as_increase()?)?
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

/// Calculates if an account will remain solvent after reducing CASH by amount and adding an amount of asset collateral.
pub fn has_liquidity_to_reduce_cash_with_added_collateral<T: Config>(
    account: ChainAccount,
    amount: CashQuantity,
    collateral_asset: AssetInfo,
    collateral_amount: AssetQuantity,
) -> Result<bool, Reason> {
    if collateral_asset.ticker != collateral_amount.units.ticker {
        Err(Reason::AssetQuantityMismatch)?
    }
    let liquidity = Portfolio::from_storage::<T>(account)?
        .cash_change(amount.as_decrease()?)?
        .asset_change(collateral_asset, collateral_amount.as_increase()?)?
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
    fn test_has_non_negative_liquidity_positive() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
            init_asset_balance(Wbtc, account, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
            init_cash(account, CashPrincipal::from_nominal("95300"));

            let given = has_non_negative_liquidity::<Test>(account);
            if given != Ok(true) {
                panic!(
                    "has_non_negative_liquidity(account): Expected={:?}, Given={:?}",
                    true, given
                );
            }
        })
    }

    #[test]
    fn test_has_non_negative_liquidity_negative() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = +4800
            init_asset_balance(Wbtc, account, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
            init_cash(account, CashPrincipal::from_nominal("95100"));

            let given = has_non_negative_liquidity::<Test>(account);
            if given != Ok(false) {
                panic!(
                    "has_non_negative_liquidity(account): Expected={:?}, Given={:?}",
                    false, given
                );
            }
        })
    }

    #[test]
    fn test_has_non_negative_liquidity_zero() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = +4800
            init_asset_balance(Wbtc, account, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
            init_cash(account, CashPrincipal::from_nominal("95200"));

            let given = has_non_negative_liquidity::<Test>(account);
            if given != Ok(true) {
                panic!(
                    "has_non_negative_liquidity(account): Expected={:?}, Given={:?}",
                    true, given
                );
            }
        })
    }

    #[test]
    fn test_has_non_negative_liquidity_empty() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            let given = has_non_negative_liquidity::<Test>(account);
            if given != Ok(true) {
                panic!(
                    "has_non_negative_liquidity(account): Expected={:?}, Given={:?}",
                    true, given
                );
            }
        })
    }

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

    // TODO: Consider what to do with unsupported assets

    #[test]
    fn test_has_liquidity_to_reduce_asset_with_fee() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800

            vec![
                (eth, Quantity::from_nominal("3", ETH), Quantity::from_nominal("0", CASH), true),
                (eth, Quantity::from_nominal("3", ETH), Quantity::from_nominal("1", CASH), false),
                (eth, Quantity::from_nominal("2.99999999", ETH), Quantity::from_nominal("1", CASH), false),
                (eth, Quantity::from_nominal("4", ETH), Quantity::from_nominal("0", CASH), false),
                (wbtc, Quantity::from_nominal("0.048", WBTC), Quantity::from_nominal("0", CASH), true), // WBTC = 4800/60000=0.08 * 0.6 = 0.048
                (wbtc, Quantity::from_nominal("0.049", WBTC), Quantity::from_nominal("0", CASH), false),
                (wbtc, Quantity::from_nominal("0.048", WBTC), Quantity::from_nominal("1", CASH), false),
                (wbtc, Quantity::from_nominal("0.04799999", WBTC), Quantity::from_nominal("1", CASH), false),
                (wbtc, Quantity::from_nominal("1", WBTC), Quantity::from_nominal("1", CASH), false),
            ]
            .into_iter()
            .for_each(|(asset, amount, fee, expected)| {
                let given = has_liquidity_to_reduce_asset_with_fee::<Test>(account, asset, amount, fee);
                if given != Ok(expected) {
                    panic!("has_liquidity_to_reduce_asset_with_fee(account, {}, {:?}, {:?}): Expected={:?}, Given={:?}", String::from(asset.symbol), amount, fee, expected, given);
                }
            })
        })
    }

    #[test]
    fn test_has_liquidity_to_reduce_asset_with_added_collateral() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800

            vec![
                (eth, Quantity::from_nominal("3", ETH), eth, Quantity::from_nominal("3", ETH), true), // net-net
                (eth, Quantity::from_nominal("4", ETH), eth, Quantity::from_nominal("1", ETH), true),
                (eth, Quantity::from_nominal("4.1", ETH), eth, Quantity::from_nominal("1", ETH), false),
                (wbtc, Quantity::from_nominal("1.001", WBTC), wbtc, Quantity::from_nominal("1", WBTC), true),
                (wbtc, Quantity::from_nominal("1", WBTC), eth, Quantity::from_nominal("59.6", ETH), true),
                (wbtc, Quantity::from_nominal("1", WBTC), eth, Quantity::from_nominal("59.5", ETH), true), // ( 60,000 / 0.6 ) = $100,000 / ( 2000 * 0.8 ) = 62.5 ETH - 3
                (wbtc, Quantity::from_nominal("1", WBTC), eth, Quantity::from_nominal("59.4", ETH), false),
            ]
            .into_iter()
            .for_each(|(asset, amount, added_asset, added_amount, expected)| {
                let given = has_liquidity_to_reduce_asset_with_added_collateral::<Test>(account, asset, amount, added_asset, added_amount);
                if given != Ok(expected) {
                    panic!("test_has_liquidity_to_reduce_asset_with_added_collateral(account, {}, {:?}, {}, {:?}): Expected={:?}, Given={:?}", String::from(asset.symbol), amount, String::from(added_asset.symbol), added_amount, expected, given);
                }
            })
        })
    }

    #[test]
    fn test_has_liquidity_to_reduce_asset_with_added_cash() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800

            vec![
                (eth, Quantity::from_nominal("3", ETH), Quantity::from_nominal("0", CASH), true),
                (eth, Quantity::from_nominal("4", ETH), Quantity::from_nominal("2500", CASH), true),
                (eth, Quantity::from_nominal("4", ETH), Quantity::from_nominal("2499", CASH), false),
                (wbtc, Quantity::from_nominal("1", WBTC), Quantity::from_nominal("95199", CASH), false), // (60000 / 0.6) - ( 3 * 2000 * 0.8 )
                (wbtc, Quantity::from_nominal("1", WBTC), Quantity::from_nominal("95200", CASH), true),
                (wbtc, Quantity::from_nominal("1", WBTC), Quantity::from_nominal("95201", CASH), true),
            ]
            .into_iter()
            .for_each(|(asset, amount, added_cash, expected)| {
                let given = has_liquidity_to_reduce_asset_with_added_cash::<Test>(account, asset, amount, added_cash);
                if given != Ok(expected) {
                    panic!("has_liquidity_to_reduce_asset_with_added_cash(account, {}, {:?}, {:?}): Expected={:?}, Given={:?}", String::from(asset.symbol), amount, added_cash, expected, given);
                }
            })
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

    #[test]
    fn test_has_liquidity_to_reduce_cash_with_added_collateral() {
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0u8; 20]);

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            init_asset_balance(Eth, account, Balance::from_nominal("1", ETH).value); // 2000 * 0.8 = 1600

            vec![
                (Quantity::from_nominal("3200", CASH), eth, Quantity::from_nominal("1", ETH), true),
                (Quantity::from_nominal("3201", CASH), eth, Quantity::from_nominal("1", ETH), false),
                (Quantity::from_nominal("1600", CASH), eth, Quantity::from_nominal("0", ETH), true),
                (Quantity::from_nominal("1601", CASH), eth, Quantity::from_nominal("0", ETH), false),
                (Quantity::from_nominal("37600", CASH), wbtc, Quantity::from_nominal("1", WBTC), true),
                (Quantity::from_nominal("37601", CASH), wbtc, Quantity::from_nominal("1", WBTC), false)
            ]
            .into_iter()
            .for_each(|(cash_amount, asset, amount, expected)| {
                let given = has_liquidity_to_reduce_cash_with_added_collateral::<Test>(account, cash_amount, asset, amount);
                if given != Ok(expected) {
                    panic!(
                        "has_liquidity_to_reduce_cash_with_added_collateral(account, {:?}, {}, {:?}): Expected={:?}, Given={:?}",
                        cash_amount, String::from(asset.symbol), amount, expected, given
                    );
                }
            })
        })
    }
}
