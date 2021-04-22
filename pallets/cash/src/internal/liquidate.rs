// Note: The substrate build requires these be re-exported.
pub use our_std::{
    cmp::{max, min},
    collections::btree_set::BTreeSet,
    convert::{TryFrom, TryInto},
    fmt, if_std, result,
    result::Result,
    str,
};

use frame_support::storage::{StorageMap, StorageValue};

use crate::{
    chains::ChainAccount,
    core::{self, get_price, get_value},
    factor::Factor,
    must,
    params::MIN_TX_VALUE,
    pipeline::CashPipeline,
    reason::Reason,
    require, require_min_tx_value,
    symbol::Units,
    types::{AssetInfo, AssetQuantity, CashPrincipalAmount, Quantity, CASH},
    Config, Event, GlobalCashIndex, Module,
};

fn calculate_seize_quantity<T: Config>(
    quantity: AssetQuantity,
    collateral_units: Units,
) -> Result<Quantity, Reason> {
    let liquidation_incentive = Factor::from_nominal("1.08"); // XXX spec first
    let asset_price = get_price::<T>(quantity.units)?;
    let collateral_price = get_price::<T>(collateral_units)?;

    if asset_price.value == 0 || collateral_price.value == 0 {
        Err(Reason::NoPrice)?
    }

    Ok(quantity
        .mul_factor(liquidation_incentive)?
        .mul_price(asset_price)?
        .div_price(collateral_price, collateral_units)?)
}

pub fn liquidate_internal<T: Config>(
    asset: AssetInfo,
    collateral_asset: AssetInfo,
    liquidator: ChainAccount,
    borrower: ChainAccount,
    quantity: AssetQuantity,
) -> Result<(), Reason> {
    require!(asset != collateral_asset, Reason::InKindLiquidation);
    require_min_tx_value!(get_value::<T>(quantity)?);
    let seize_quantity = calculate_seize_quantity::<T>(quantity, collateral_asset.units())?;

    CashPipeline::new()
        .check_underwater::<T>(borrower)?
        .transfer_asset::<T>(liquidator, borrower, asset.asset, quantity)?
        .transfer_asset::<T>(borrower, liquidator, collateral_asset.asset, seize_quantity)?
        .check_asset_balance::<T, _>(borrower, asset, |asset_balance| {
            must!(asset_balance.lte(0), Reason::RepayTooMuch)
        })?
        .check_asset_balance::<T, _>(borrower, collateral_asset, |collateral_balance| {
            must!(collateral_balance.gte(0), Reason::InsufficientCollateral)
        })?
        .check_collateralized::<T>(liquidator)?
        .commit::<T>();

    <Module<T>>::deposit_event(Event::Liquidate(
        asset.asset,
        collateral_asset.asset,
        liquidator,
        borrower,
        quantity.value,
    ));

    Ok(())
}

pub fn liquidate_cash_principal_internal<T: Config>(
    collateral_asset: AssetInfo,
    liquidator: ChainAccount,
    borrower: ChainAccount,
    principal: CashPrincipalAmount,
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();
    let quantity = index.cash_quantity(principal)?;

    require_min_tx_value!(get_value::<T>(quantity)?);
    let seize_quantity = calculate_seize_quantity::<T>(quantity, collateral_asset.units())?;

    CashPipeline::new()
        .check_underwater::<T>(borrower)?
        .transfer_cash::<T>(liquidator, borrower, principal)?
        .transfer_asset::<T>(borrower, liquidator, collateral_asset.asset, seize_quantity)?
        .check_cash_principal::<T, _>(borrower, |cash_principal| {
            must!(cash_principal.lte(0), Reason::RepayTooMuch)
        })?
        .check_asset_balance::<T, _>(borrower, collateral_asset, |collateral_balance| {
            must!(collateral_balance.gte(0), Reason::InsufficientCollateral)
        })?
        .check_collateralized::<T>(liquidator)?
        .commit::<T>();

    <Module<T>>::deposit_event(Event::LiquidateCash(
        collateral_asset.asset,
        liquidator,
        borrower,
        principal,
        index,
    ));

    Ok(())
}

pub fn liquidate_cash_collateral_internal<T: Config>(
    asset: AssetInfo,
    liquidator: ChainAccount,
    borrower: ChainAccount,
    quantity: AssetQuantity,
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();

    require_min_tx_value!(get_value::<T>(quantity)?);
    let seize_quantity = calculate_seize_quantity::<T>(quantity, CASH)?;
    let seize_principal = index.cash_principal_amount(seize_quantity)?;

    CashPipeline::new()
        .check_underwater::<T>(borrower)?
        .transfer_asset::<T>(liquidator, borrower, asset.asset, quantity)?
        .transfer_cash::<T>(borrower, liquidator, seize_principal)?
        .check_asset_balance::<T, _>(borrower, asset, |asset_balance| {
            must!(asset_balance.lte(0), Reason::RepayTooMuch)
        })?
        .check_cash_principal::<T, _>(borrower, |cash_principal| {
            must!(cash_principal.gte(0), Reason::InsufficientCollateral)
        })?
        .check_collateralized::<T>(liquidator)?
        .commit::<T>();

    <Module<T>>::deposit_event(Event::LiquidateCashCollateral(
        asset.asset,
        liquidator,
        borrower,
        quantity.value,
    ));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        tests::{assert_ok, assets::*, common::*, mock::*},
        types::*,
        *,
    };
    use pallet_oracle::types::Price;

    #[allow(non_upper_case_globals)]
    const asset: AssetInfo = eth;
    #[allow(non_upper_case_globals)]
    const collateral_asset: AssetInfo = wbtc;
    #[allow(non_upper_case_globals)]
    const liquidator: ChainAccount = ChainAccount::Eth([1u8; 20]);
    #[allow(non_upper_case_globals)]
    const borrower: ChainAccount = ChainAccount::Eth([2u8; 20]);

    #[test]
    fn test_calculate_seize_quantity_no_asset_price() {
        new_test_ext().execute_with(|| {
            let quantity: AssetQuantity = eth.as_quantity_nominal("1");

            init_wbtc_asset().unwrap();

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, WBTC),
                Err(Reason::NoPrice)
            );
        })
    }

    #[test]
    fn test_calculate_seize_quantity_no_collateral_asset_price() {
        new_test_ext().execute_with(|| {
            let quantity: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, WBTC),
                Err(Reason::NoPrice)
            );
        })
    }

    #[test]
    fn test_calculate_seize_quantity_overflow_incentive() {
        new_test_ext().execute_with(|| {
            let quantity: AssetQuantity = Quantity {
                value: u128::MAX,
                units: ETH,
            };

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, WBTC),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    #[test]
    fn test_calculate_seize_quantity_overflow_price() {
        new_test_ext().execute_with(|| {
            let quantity: AssetQuantity = Quantity {
                value: u128::MAX / 10,
                units: ETH,
            };

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, WBTC),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    #[test]
    fn test_calculate_seize_quantity_zero_asset_price() {
        new_test_ext().execute_with(|| {
            let quantity: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            pallet_oracle::Prices::insert(ETH.ticker, Price::from_nominal(ETH.ticker, "0").value);

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, WBTC),
                Err(Reason::NoPrice)
            );
        })
    }

    #[test]
    fn test_calculate_seize_quantity_zero_price() {
        new_test_ext().execute_with(|| {
            let quantity: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            pallet_oracle::Prices::insert(WBTC.ticker, Price::from_nominal(WBTC.ticker, "0").value);

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, WBTC),
                Err(Reason::NoPrice)
            );
        })
    }

    #[test]
    fn test_calculate_seize_quantity_ok() {
        new_test_ext().execute_with(|| {
            let quantity: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();
            pallet_oracle::Prices::insert(
                ETH.ticker,
                Price::from_nominal(ETH.ticker, "2000").value,
            );
            pallet_oracle::Prices::insert(
                WBTC.ticker,
                Price::from_nominal(WBTC.ticker, "60000").value,
            );

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, WBTC),
                Ok(Quantity {
                    value: 3600000, // 1.08 * 1 * 2000 / 60000 = 0.036e8
                    units: WBTC
                })
            );

            pallet_oracle::Prices::insert(
                WBTC.ticker,
                Price::from_nominal(WBTC.ticker, "50000").value,
            );

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, WBTC),
                Ok(Quantity {
                    value: 4320000, // 1.08 * 1 * 2000 / 50000 = 0.0432e8
                    units: WBTC
                })
            );

            assert_eq!(
                calculate_seize_quantity::<Test>(quantity, CASH),
                Ok(Quantity {
                    value: 2160000000, // 1.08 * 1 * 2000 / 1 = 2160e6
                    units: CASH
                })
            );

            assert_eq!(
                calculate_seize_quantity::<Test>(
                    Quantity {
                        value: 1000000000,
                        units: CASH
                    },
                    ETH
                ),
                Ok(Quantity {
                    value: 540000000000000000, // 1.08 * 1 * 1000 / 2000 = 0.54e18
                    units: ETH
                })
            );
        })
    }

    // liquidate_internal

    #[test]
    fn test_liquidate_internal_self_liquidate() {
        new_test_ext().execute_with(|| {
            init_eth_asset().unwrap();
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, borrower, borrower, amount),
                Err(Reason::SelfTransfer)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_in_kind_liquidation() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            assert_eq!(
                liquidate_internal::<Test>(asset, asset, liquidator, borrower, amount),
                Err(Reason::InKindLiquidation)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_too_small() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("0.00000001");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::MinTxValueNotMet)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_too_large() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = Quantity {
                value: u128::MAX,
                units: ETH,
            };

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    // The hope on this test is to hit the overflow on mul'ing by price,
    // not on mul'ing by the liquidation incentive (1.08).
    #[test]
    fn test_liquidate_internal_too_large_with_price() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = Quantity {
                value: u128::MAX / 10 * 9,
                units: ETH,
            };

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    #[test]
    fn test_liquidate_internal_not_supported() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            pallet_oracle::Prices::insert(
                ETH.ticker,
                Price::from_nominal(ETH.ticker, "2000.00").value,
            );
            pallet_oracle::Prices::insert(
                WBTC.ticker,
                Price::from_nominal(WBTC.ticker, "60000.00").value,
            );
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000

            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::AssetNotSupported)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_not_supported_collateral() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            pallet_oracle::Prices::insert(
                ETH.ticker,
                Price::from_nominal(ETH.ticker, "2000.00").value,
            );
            pallet_oracle::Prices::insert(
                WBTC.ticker,
                Price::from_nominal(WBTC.ticker, "60000.00").value,
            );
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::AssetNotSupported)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_asset_price_zero() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            pallet_oracle::Prices::insert(ETH.ticker, Price::from_nominal(ETH.ticker, "0").value);

            // This will always trip first
            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::MinTxValueNotMet)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_collateral_asset_price_zero() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            pallet_oracle::Prices::insert(WBTC.ticker, Price::from_nominal(WBTC.ticker, "0").value);

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::NoPrice)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_sufficient_liquidity() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
            init_cash(borrower, CashPrincipal::from_nominal("97000")); // 97000 + 4800 - 1000000 = 1800

            // Seize amount = 1.08 * 1 * 2000 / 60000 = 0.036e8

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::SufficientLiquidity)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_insufficient_liquidity() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-80", ETH).value); // -80 * 2000 / 0.8 = -200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // 100000 + 72000 - 200000 = -28000

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::InsufficientLiquidity)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_repay_too_much() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("90");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-80", ETH).value); // -80 * 2000 / 0.8 = -200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // 100000 + 72000 - 200000 = -28000

            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::RepayTooMuch)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_repay_too_much_zero_balance_asset() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = uni.as_quantity_nominal("100");

            init_uni_asset().unwrap();
            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-80", ETH).value); // -80 * 2000 / 0.8 = -200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // 100000 + 72000 - 200000 = -28000

            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_eq!(
                liquidate_internal::<Test>(uni, collateral_asset, liquidator, borrower, amount),
                Err(Reason::RepayTooMuch)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_repay_too_much_positive_balance_asset() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = uni.as_quantity_nominal("100");

            init_uni_asset().unwrap();
            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Uni, borrower, Balance::from_nominal("100", UNI).value);
            init_asset_balance(Eth, borrower, Balance::from_nominal("-8000", ETH).value);
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value);

            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_eq!(
                liquidate_internal::<Test>(uni, collateral_asset, liquidator, borrower, amount),
                Err(Reason::RepayTooMuch)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_insufficient_collateral() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-80", ETH).value); // -80 * 2000 / 0.8 = -200000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // 100000 + 200000 = -100000

            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::InsufficientCollateral)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_insufficient_collateral_partial() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("10");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-80", ETH).value); // -80 * 2000 / 0.8 = -200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("0.01", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // 100000 + 200000 = -100000

            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::InsufficientCollateral)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_ok() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-80", ETH).value); // -80 * 2000 / 0.8 = -200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // 100000 + 72000 - 200000 = -28000

            // Seize amount = 1.08 * 1 * 2000 / 60000 = 0.036 WBTC

            init_asset_balance(Wbtc, liquidator, Balance::from_nominal("1", WBTC).value);
            init_asset_balance(Eth, liquidator, Balance::from_nominal("0.5", ETH).value);
            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_ok!(liquidate_internal::<Test>(
                asset,
                collateral_asset,
                liquidator,
                borrower,
                amount
            ));

            assert_eq!(
                TotalSupplyAssets::get(Eth),
                Quantity::from_nominal("0", ETH).value
            ); // All supply was closed
            assert_eq!(
                TotalBorrowAssets::get(Eth),
                Quantity::from_nominal("79.5", ETH).value
            ); // Part of liquidation was borrowed, part came from supply
            assert_eq!(
                TotalSupplyAssets::get(Wbtc),
                Quantity::from_nominal("3", WBTC).value
            );
            assert_eq!(
                TotalBorrowAssets::get(Wbtc),
                Quantity::from_nominal("0", WBTC).value
            );
            assert_eq!(
                AssetBalances::get(Eth, borrower),
                Balance::from_nominal("-79", ETH).value
            );
            assert_eq!(
                AssetBalances::get(Eth, liquidator),
                Balance::from_nominal("-0.5", ETH).value
            );
            assert_eq!(
                AssetBalances::get(Wbtc, borrower),
                Balance::from_nominal("1.964", WBTC).value
            );
            assert_eq!(
                AssetBalances::get(Wbtc, liquidator),
                Balance::from_nominal("1.036", WBTC).value
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(borrower).collect::<Vec<_>>(),
                vec![(Eth, ()), (Wbtc, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(liquidator).collect::<Vec<_>>(),
                vec![(Eth, ()), (Wbtc, ())]
            );
            assert_eq!(
                LastIndices::get(Eth, borrower),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Eth, liquidator),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Wbtc, borrower),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Wbtc, liquidator),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(borrower),
                CashPrincipal::from_nominal("100000")
            );
            assert_eq!(
                CashPrincipals::get(liquidator),
                CashPrincipal::from_nominal("100000")
            );
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("200000")
            );
        })
    }

    // liquidate_cash_principal

    #[test]
    fn test_liquidate_cash_principal_internal_self_liquidate() {
        new_test_ext().execute_with(|| {
            init_eth_asset().unwrap();
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("100");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    borrower,
                    borrower,
                    principal
                ),
                Err(Reason::SelfTransfer)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_too_small() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("0.99");

            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::MinTxValueNotMet)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_too_large() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount(u128::MAX);

            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    // The hope on this test is to hit the overflow on mul'ing by price,
    // not on mul'ing by the liquidation incentive (1.08).
    #[test]
    fn test_liquidate_cash_principal_internal_too_large_with_price() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount(u128::MAX / 10 * 9);

            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_not_supported_collateral() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("100");

            pallet_oracle::Prices::insert(
                WBTC.ticker,
                Price::from_nominal(WBTC.ticker, "60000.00").value,
            );
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::AssetNotSupported)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_collateral_asset_price_zero() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("100");

            init_wbtc_asset().unwrap();

            pallet_oracle::Prices::insert(WBTC.ticker, Price::from_nominal(WBTC.ticker, "0").value);

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::NoPrice)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_sufficient_liquidity() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("100");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
            init_cash(borrower, CashPrincipal::from_nominal("97000")); // 97000 + 4800 - 1000000 = 1800

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::SufficientLiquidity)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_insufficient_liquidity() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("100");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("80", ETH).value); // 80 * 2000 / 0.8 = 200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("-300000")); // -300000 + 72000 + 200000 = -28000

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::InsufficientLiquidity)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_repay_too_much() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("350000");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("80", ETH).value); // 80 * 2000 / 0.8 = 200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("-300000")); // -300000 + 72000 + 200000 = -28000

            init_cash(liquidator, CashPrincipal::from_nominal("400000"));

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::RepayTooMuch)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_repay_too_much_positive_balance_asset() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("100");

            init_uni_asset().unwrap();
            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Uni, borrower, Balance::from_nominal("100", UNI).value);
            init_asset_balance(Eth, borrower, Balance::from_nominal("-8000", ETH).value);
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value);

            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::RepayTooMuch)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_insufficient_collateral() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("100");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("80", ETH).value); // -80 * 2000 / 0.8 = -200000
            init_cash(borrower, CashPrincipal::from_nominal("-300000")); // -300000 + 200000 = -100000

            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::InsufficientCollateral)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_insufficient_collateral_partial() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("10000");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("80", ETH).value); // 80 * 2000 / 0.8 = 200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("0.01", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("-300000"));

            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            assert_eq!(
                liquidate_cash_principal_internal::<Test>(
                    collateral_asset,
                    liquidator,
                    borrower,
                    principal
                ),
                Err(Reason::InsufficientCollateral)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_principal_internal_ok() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("60000");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("80", ETH).value); // 80 * 2000 / 0.8 = 200000
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("2", WBTC).value); // 2 * 60000 * 0.6 = 72000
            init_cash(borrower, CashPrincipal::from_nominal("-300000")); // -300000 + 72000 + 200000 = -28000

            // Seize amount = 1.08 * 1 * 60000 / 60000 = 1.08 WBTC

            init_asset_balance(Wbtc, liquidator, Balance::from_nominal("-0.1", WBTC).value);
            init_cash(liquidator, CashPrincipal::from_nominal("100000"));

            // TODO: Check this number
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("100000")
            );

            assert_ok!(liquidate_cash_principal_internal::<Test>(
                collateral_asset,
                liquidator,
                borrower,
                principal
            ));

            assert_eq!(
                TotalSupplyAssets::get(Wbtc),
                Quantity::from_nominal("1.9", WBTC).value
            );
            assert_eq!(
                TotalBorrowAssets::get(Wbtc),
                Quantity::from_nominal("0", WBTC).value
            );
            assert_eq!(
                AssetBalances::get(Wbtc, borrower),
                Balance::from_nominal("0.92", WBTC).value
            );
            assert_eq!(
                AssetBalances::get(Wbtc, liquidator),
                Balance::from_nominal("0.98", WBTC).value
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(borrower).collect::<Vec<_>>(),
                vec![(Eth, ()), (Wbtc, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(liquidator).collect::<Vec<_>>(),
                vec![(Wbtc, ())]
            );
            assert_eq!(
                LastIndices::get(Wbtc, borrower),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Wbtc, liquidator),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(ChainAccount::Eth([0; 20])),
                CashPrincipal::from_nominal("300000")
            );
            assert_eq!(
                CashPrincipals::get(borrower),
                CashPrincipal::from_nominal("-240000")
            );
            assert_eq!(
                CashPrincipals::get(liquidator),
                CashPrincipal::from_nominal("40000")
            );
            // TODO: Check this number
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("40000")
            );
        })
    }

    // liquidate_cash_collateral

    #[test]
    fn test_liquidate_cash_collateral_internal_self_liquidate() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-40", ETH).value); // 40 * 2000 / 0.8 = -200000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // -200000 + 100000 = -100000

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, borrower, borrower, amount),
                Err(Reason::SelfTransfer)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_too_small() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("0.00000001");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::MinTxValueNotMet)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_too_large() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = Quantity {
                value: u128::MAX,
                units: ETH,
            };

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    // The hope on this test is to hit the overflow on mul'ing by price,
    // not on mul'ing by the liquidation incentive (1.08).
    #[test]
    fn test_liquidate_cash_collateral_internal_too_large_with_price() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = Quantity {
                value: u128::MAX / 10 * 9,
                units: ETH,
            };

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_not_supported() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            pallet_oracle::Prices::insert(
                ETH.ticker,
                Price::from_nominal(ETH.ticker, "2000.00").value,
            );

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::AssetNotSupported)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_asset_price_zero() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();

            pallet_oracle::Prices::insert(ETH.ticker, Price::from_nominal(ETH.ticker, "0").value);

            // This will always trip first
            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::MinTxValueNotMet)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_sufficient_liquidity() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-40", ETH).value); // 40 * 2000 / 0.8 = -200000
            init_cash(borrower, CashPrincipal::from_nominal("1000000")); // -200000 + 1000000 = 800000

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::SufficientLiquidity)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_insufficient_liquidity() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-40", ETH).value); // 40 * 2000 / 0.8 = -200000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // -200000 + 100000 = -100000

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::InsufficientLiquidity)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_repay_too_much() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("50");

            init_eth_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-40", ETH).value); // 40 * 2000 / 0.8 = -200000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // -200000 + 100000 = -100000

            init_asset_balance(Eth, liquidator, Balance::from_nominal("80", ETH).value);

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::RepayTooMuch)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_repay_too_much_zero_balance_asset() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = uni.as_quantity_nominal("100");

            init_uni_asset().unwrap();
            init_eth_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-40", ETH).value); // 40 * 2000 / 0.8 = -200000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // -200000 + 100000 = -100000

            init_asset_balance(Eth, liquidator, Balance::from_nominal("80", ETH).value);

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(uni, liquidator, borrower, amount),
                Err(Reason::RepayTooMuch)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_repay_too_much_positive_balance_asset() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = uni.as_quantity_nominal("100");

            init_uni_asset().unwrap();
            init_eth_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-4000", ETH).value);
            init_asset_balance(Uni, borrower, Balance::from_nominal("100", UNI).value);
            init_cash(borrower, CashPrincipal::from_nominal("100000"));

            init_asset_balance(Eth, liquidator, Balance::from_nominal("80", ETH).value);

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(uni, liquidator, borrower, amount),
                Err(Reason::RepayTooMuch)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_insufficient_collateral() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-40", ETH).value); // 40 * 2000 / 0.8 = -200000
            init_asset_balance(Eth, liquidator, Balance::from_nominal("80", ETH).value);

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::InsufficientCollateral)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_insufficient_collateral_partial() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("10");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-40", ETH).value);
            init_cash(borrower, CashPrincipal::from_nominal("100"));

            init_asset_balance(Eth, liquidator, Balance::from_nominal("80", ETH).value);

            assert_eq!(
                liquidate_cash_collateral_internal::<Test>(asset, liquidator, borrower, amount),
                Err(Reason::InsufficientCollateral)
            );
        })
    }

    #[test]
    fn test_liquidate_cash_collateral_internal_ok() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("10");

            init_eth_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-40", ETH).value); // 40 * 2000 / 0.8 = -200000
            init_cash(borrower, CashPrincipal::from_nominal("100000")); // -200000 + 100000 = -100000

            init_asset_balance(Eth, liquidator, Balance::from_nominal("80", ETH).value);
            init_cash(liquidator, CashPrincipal::from_nominal("-600"));

            // Seize amount = 1.08 * 10 * 2000 / 1 = 21600 CASH

            assert_ok!(liquidate_cash_collateral_internal::<Test>(
                asset, liquidator, borrower, amount
            ));

            assert_eq!(
                TotalSupplyAssets::get(Eth),
                Quantity::from_nominal("70", ETH).value
            ); // All supply was closed
            assert_eq!(
                TotalBorrowAssets::get(Eth),
                Quantity::from_nominal("30", ETH).value
            ); // Part of liquidation was borrowed, part came from supply
            assert_eq!(
                AssetBalances::get(Eth, borrower),
                Balance::from_nominal("-30", ETH).value
            );
            assert_eq!(
                AssetBalances::get(Eth, liquidator),
                Balance::from_nominal("70", ETH).value
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(borrower).collect::<Vec<_>>(),
                vec![(Eth, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(liquidator).collect::<Vec<_>>(),
                vec![(Eth, ())]
            );
            assert_eq!(
                LastIndices::get(Eth, borrower),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Eth, liquidator),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(borrower),
                CashPrincipal::from_nominal("78400")
            );
            assert_eq!(
                CashPrincipals::get(liquidator),
                CashPrincipal::from_nominal("21000")
            );
            // TODO: Check this number
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("99400")
            );
        })
    }
}
