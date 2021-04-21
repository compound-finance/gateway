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
    internal::liquidity,
    params::MIN_TX_VALUE,
    pipeline::CashPipeline,
    reason::Reason,
    require, require_min_tx_value,
    types::{AssetInfo, AssetQuantity, CashPrincipalAmount, CASH},
    Config, Event, GlobalCashIndex, Module, TotalBorrowAssets, TotalSupplyAssets,
};

pub fn liquidate_internal<T: Config>(
    asset: AssetInfo,
    collateral_asset: AssetInfo,
    liquidator: ChainAccount,
    borrower: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    require!(asset != collateral_asset, Reason::InKindLiquidation); // TODO: Can we allow this now?
    require_min_tx_value!(get_value::<T>(amount)?);

    let liquidation_incentive = Factor::from_nominal("1.08"); // XXX spec first
    let seize_amount = amount
        .mul_factor(liquidation_incentive)?
        .mul_price(get_price::<T>(asset.units())?)?
        .div_price(
            get_price::<T>(collateral_asset.units())?,
            collateral_asset.units(),
        )?;

    require!(
        !liquidity::has_non_negative_liquidity::<T>(borrower)?,
        Reason::SufficientLiquidity
    );

    CashPipeline::new()
        .transfer_asset::<T>(liquidator, borrower, asset.asset, amount)?
        .transfer_asset::<T>(borrower, liquidator, collateral_asset.asset, seize_amount)?
        .check_collateralized::<T>(liquidator)?
        // Note: don't check borrower liquidity
        .commit::<T>();

    <Module<T>>::deposit_event(Event::Liquidate(
        asset.asset,
        collateral_asset.asset,
        liquidator,
        borrower,
        amount.value,
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
    let amount = index.cash_quantity(principal)?;

    require_min_tx_value!(get_value::<T>(amount)?);
    require!(
        !liquidity::has_non_negative_liquidity::<T>(borrower)?,
        Reason::SufficientLiquidity
    );

    let liquidation_incentive = Factor::from_nominal("1.08"); // XXX spec first
    let seize_amount = amount
        .mul_factor(liquidation_incentive)?
        .mul_price(get_price::<T>(CASH)?)?
        .div_price(
            get_price::<T>(collateral_asset.units())?,
            collateral_asset.units(),
        )?;

    CashPipeline::new()
        .transfer_cash::<T>(liquidator, borrower, principal)?
        .transfer_asset::<T>(borrower, liquidator, collateral_asset.asset, seize_amount)?
        .check_collateralized::<T>(liquidator)?
        // Note: don't check borrower liquidity
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
    amount: AssetQuantity,
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();

    require_min_tx_value!(get_value::<T>(amount)?);

    require!(
        !liquidity::has_non_negative_liquidity::<T>(borrower)?,
        Reason::SufficientLiquidity
    );

    let liquidation_incentive = Factor::from_nominal("1.08"); // XXX spec first
    let seize_amount = amount
        .mul_factor(liquidation_incentive)?
        .mul_price(get_price::<T>(asset.units())?)?
        .div_price(get_price::<T>(CASH)?, CASH)?;
    let seize_principal = index.cash_principal_amount(seize_amount)?;

    CashPipeline::new()
        .transfer_asset::<T>(liquidator, borrower, asset.asset, amount)?
        .transfer_cash::<T>(borrower, liquidator, seize_principal)?
        .check_collateralized::<T>(liquidator)?
        // Note: don't check borrower liquidity
        .commit::<T>();

    <Module<T>>::deposit_event(Event::LiquidateCashCollateral(
        asset.asset,
        liquidator,
        borrower,
        amount.value,
    ));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::*,
        tests::{assets::*, common::*, mock::*},
        types::*,
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
    fn test_liquidate_internal_no_asset_price() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_wbtc_asset().unwrap();

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::NoPrice)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_no_collateral_asset_price() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::NoPrice)
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
                Err(Reason::MathError(MathError::DivisionByZero))
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

            init_asset_balance(Eth, borrower, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
            init_cash(borrower, CashPrincipal::from_nominal("95000")); // 95000 + 4800 - 1000000 = -200

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::InsufficientLiquidity)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_borrower_asset_overflow() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, i128::MAX);
            init_asset_balance(Wbtc, borrower, i128::MIN + 1);

            init_cash(liquidator, CashPrincipal::from_nominal("1000000"));

            // I'm not entirely sure this is caused by the line I want,
            // but it might be!
            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    // #[test]
    // fn test_liquidate_internal_liquidator_asset_underflow() {
    //     new_test_ext().execute_with(|| {
    //         // let amount: AssetQuantity = Quantity {
    //         //     value: 2500,
    //         //     units: ETH,
    //         // };
    //         let amount: AssetQuantity = eth.as_quantity_nominal("1");

    //         pallet_oracle::Prices::insert(ETH.ticker, 1);

    //         init_eth_asset().unwrap();
    //         init_wbtc_asset().unwrap();

    //         init_asset_balance(Eth, borrower, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
    //         init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
    //         init_cash(borrower, CashPrincipal::from_nominal("95000")); // 95000 + 4800 - 1000000 = -200

    //         init_asset_balance(Eth, borrower, i128::MIN / 100000);
    //         init_cash(liquidator, CashPrincipal::from_nominal("1000000"));

    //         // I'm not entirely sure this is caused by the line I want,
    //         // but it might be!
    //         assert_eq!(
    //             liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
    //             Err(Reason::MathError(MathError::Overflow))
    //         );
    //     })
    // }

    #[test]
    fn test_liquidate_internal_total_supply_underflow() {
        /* The underwater user has -3 Eth the liquidator has 3 Eth, but the protocol magically
           has zero total supplied Eth. The liquidation causes a net-loss of Eth and thus underflows.
           This scenario cannot happen in normal conditions due to the set-up
        */
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("-3", ETH).value);
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value);

            init_cash(liquidator, CashPrincipal::from_nominal("1000000"));

            init_asset_balance(Eth, liquidator, Balance::from_nominal("3", ETH).value);
            TotalSupplyAssets::insert(asset.asset, 0);

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::InsufficientTotalFunds)
            );
        })
    }

    #[test]
    fn test_liquidate_internal_total_borrow_assets_overflow() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = eth.as_quantity_nominal("1");

            init_eth_asset().unwrap();
            init_wbtc_asset().unwrap();

            init_asset_balance(Eth, borrower, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
            init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
            init_cash(borrower, CashPrincipal::from_nominal("95000")); // 95000 + 4800 - 1000000 = -200

            init_cash(liquidator, CashPrincipal::from_nominal("1000000"));

            TotalBorrowAssets::insert(asset.asset, u128::MAX);

            assert_eq!(
                liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
                Err(Reason::MathError(MathError::Overflow))
            );
        })
    }

    // #[test]
    // fn test_liquidate_internal_xxx() {
    //     new_test_ext().execute_with(|| {
    //         let amount: AssetQuantity = eth.as_quantity_nominal("1");

    //         init_eth_asset().unwrap();
    //         init_wbtc_asset().unwrap();

    //         init_asset_balance(Eth, borrower, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
    //         init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
    //         init_cash(borrower, CashPrincipal::from_nominal("95000")); // 95000 + 4800 - 1000000 = -200

    //         init_cash(liquidator, CashPrincipal::from_nominal("1000000"));

    //         assert_eq!(
    //             liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
    //             Err(Reason::MathError(MathError::DivisionByZero))
    //         );
    //     })
    // }

    // #[test]
    // fn test_liquidate_internal_ok() {
    //     new_test_ext().execute_with(|| {
    //         let asset: AssetInfo = eth;
    //         let collateral_asset: AssetInfo = wbtc;
    //         let liquidator: ChainAccount = ChainAccount::Eth([1u8; 20]);
    //         let borrower: ChainAccount = ChainAccount::Eth([2u8; 20]);
    //         let amount: AssetQuantity = eth.as_quantity_nominal("1");

    //         init_eth_asset().unwrap();
    //         init_wbtc_asset().unwrap();
    //         init_asset_balance(Eth, borrower, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
    //         init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
    //         init_cash(borrower, CashPrincipal::from_nominal("95000")); // 95000 + 4800 - 1000000 = -200

    //         assert_ok!(liquidate_internal::<Test>(
    //             asset,
    //             collateral_asset,
    //             liquidator,
    //             borrower,
    //             amount
    //         ));

    //         // TODO: Checks for last indices and other written values

    //         assert_eq!(
    //             AssetBalances::get(Eth, liquidator),
    //             Balance::from_nominal("32.4", ETH).value
    //         );
    //         assert_eq!(
    //             AssetBalances::get(Eth, borrower),
    //             Balance::from_nominal("-32.4", ETH).value
    //         );
    //         assert_eq!(
    //             AssetBalances::get(Wbtc, liquidator),
    //             Balance::from_nominal("2", WBTC).value
    //         );
    //         assert_eq!(
    //             AssetBalances::get(Wbtc, borrower),
    //             Balance::from_nominal("-4", WBTC).value
    //         );
    //     })
    // }
}
