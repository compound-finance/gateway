// Note: The substrate build requires these be re-exported.
pub use our_std::{
    cmp::{max, min},
    collections::btree_set::BTreeSet,
    convert::{TryFrom, TryInto},
    fmt, if_std, result,
    result::Result,
    str,
};

use frame_support::storage::{StorageDoubleMap, StorageMap, StorageValue};

use crate::{
    chains::ChainAccount,
    core::{self, get_price, get_value},
    factor::Factor,
    internal::{balance_helpers::*, liquidity},
    params::MIN_TX_VALUE,
    reason::Reason,
    require, require_min_tx_value,
    types::{AssetInfo, AssetQuantity, CashPrincipalAmount, CASH},
    AssetBalances, CashPrincipals, Config, Event, GlobalCashIndex, LastIndices, Module,
    TotalBorrowAssets, TotalCashPrincipal, TotalSupplyAssets,
};

pub fn liquidate_internal<T: Config>(
    asset: AssetInfo,
    collateral_asset: AssetInfo,
    liquidator: ChainAccount,
    borrower: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    require!(borrower != liquidator, Reason::SelfTransfer);
    require!(asset != collateral_asset, Reason::InKindLiquidation);
    require_min_tx_value!(get_value::<T>(amount)?);

    let liquidation_incentive = Factor::from_nominal("1.08"); // XXX spec first
    let liquidator_asset = AssetBalances::get(asset.asset, liquidator);
    let borrower_asset = AssetBalances::get(asset.asset, borrower);
    let liquidator_collateral_asset = AssetBalances::get(collateral_asset.asset, liquidator);
    let borrower_collateral_asset = AssetBalances::get(collateral_asset.asset, borrower);
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
    require!(
        liquidity::has_liquidity_to_reduce_asset_with_added_collateral::<T>(
            liquidator,
            asset,
            amount,
            collateral_asset,
            seize_amount,
        )?,
        Reason::InsufficientLiquidity
    );

    let (borrower_repay_amount, _borrower_supply_amount) =
        repay_and_supply_amount(liquidator_asset, amount)?;
    let (liquidator_withdraw_amount, liquidator_borrow_amount) =
        withdraw_and_borrow_amount(borrower_asset, amount)?;
    let (borrower_collateral_withdraw_amount, _borrower_collateral_borrow_amount) =
        withdraw_and_borrow_amount(borrower_collateral_asset, seize_amount)?;
    let (liquidator_collateral_repay_amount, liquidator_collateral_supply_amount) =
        repay_and_supply_amount(liquidator_collateral_asset, seize_amount)?;

    let borrower_asset_new = add_amount_to_balance(borrower_asset, amount)?;
    let liquidator_asset_new = sub_amount_from_balance(liquidator_asset, amount)?;
    let borrower_collateral_asset_new =
        sub_amount_from_balance(borrower_collateral_asset, seize_amount)?;
    let liquidator_collateral_asset_new =
        add_amount_to_balance(liquidator_collateral_asset, seize_amount)?;

    let total_supply_new = sub_amount_from_raw(
        TotalSupplyAssets::get(asset.asset),
        liquidator_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_borrow_new = sub_amount_from_raw(
        add_amount_to_raw(
            TotalBorrowAssets::get(asset.asset),
            liquidator_borrow_amount,
        )?,
        borrower_repay_amount,
        Reason::RepayTooMuch,
    )?;
    let total_collateral_supply_new = sub_amount_from_raw(
        add_amount_to_raw(
            TotalSupplyAssets::get(collateral_asset.asset),
            liquidator_collateral_supply_amount,
        )?,
        borrower_collateral_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_collateral_borrow_new = sub_amount_from_raw(
        TotalBorrowAssets::get(collateral_asset.asset),
        liquidator_collateral_repay_amount,
        Reason::RepayTooMuch,
    )?;

    let (borrower_cash_principal_post, borrower_last_index_post) =
        core::effect_of_asset_interest_internal(
            asset,
            borrower,
            borrower_asset,
            borrower_asset_new,
            CashPrincipals::get(borrower),
        )?;
    let (liquidator_cash_principal_post, liquidator_last_index_post) =
        core::effect_of_asset_interest_internal(
            asset,
            liquidator,
            liquidator_asset,
            liquidator_asset_new,
            CashPrincipals::get(liquidator),
        )?;
    let (borrower_cash_principal_post, borrower_collateral_last_index_post) =
        core::effect_of_asset_interest_internal(
            collateral_asset,
            borrower,
            borrower_collateral_asset,
            borrower_collateral_asset_new,
            borrower_cash_principal_post,
        )?;
    let (liquidator_cash_principal_post, liquidator_collateral_last_index_post) =
        core::effect_of_asset_interest_internal(
            collateral_asset,
            liquidator,
            liquidator_collateral_asset,
            liquidator_collateral_asset_new,
            liquidator_cash_principal_post,
        )?;

    LastIndices::insert(asset.asset, borrower, borrower_last_index_post);
    LastIndices::insert(asset.asset, liquidator, liquidator_last_index_post);
    LastIndices::insert(
        collateral_asset.asset,
        borrower,
        borrower_collateral_last_index_post,
    );
    LastIndices::insert(
        collateral_asset.asset,
        liquidator,
        liquidator_collateral_last_index_post,
    );
    CashPrincipals::insert(borrower, borrower_cash_principal_post);
    CashPrincipals::insert(liquidator, liquidator_cash_principal_post);
    TotalSupplyAssets::insert(asset.asset, total_supply_new);
    TotalBorrowAssets::insert(asset.asset, total_borrow_new);
    TotalSupplyAssets::insert(collateral_asset.asset, total_collateral_supply_new);
    TotalBorrowAssets::insert(collateral_asset.asset, total_collateral_borrow_new);

    core::set_asset_balance_internal::<T>(asset.asset, borrower, borrower_asset_new);
    core::set_asset_balance_internal::<T>(asset.asset, liquidator, liquidator_asset_new);
    core::set_asset_balance_internal::<T>(
        collateral_asset.asset,
        borrower,
        borrower_collateral_asset_new,
    );
    core::set_asset_balance_internal::<T>(
        collateral_asset.asset,
        liquidator,
        liquidator_collateral_asset_new,
    );

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

    require!(borrower != liquidator, Reason::SelfTransfer);
    require_min_tx_value!(get_value::<T>(amount)?);

    let liquidation_incentive = Factor::from_nominal("1.08"); // XXX spec first
    let liquidator_cash_principal = CashPrincipals::get(liquidator);
    let borrower_cash_principal = CashPrincipals::get(borrower);
    let liquidator_collateral_asset = AssetBalances::get(collateral_asset.asset, liquidator);
    let borrower_collateral_asset = AssetBalances::get(collateral_asset.asset, borrower);
    let seize_amount = amount
        .mul_factor(liquidation_incentive)?
        .mul_price(get_price::<T>(CASH)?)?
        .div_price(
            get_price::<T>(collateral_asset.units())?,
            collateral_asset.units(),
        )?;

    require!(
        !liquidity::has_non_negative_liquidity::<T>(borrower)?,
        Reason::SufficientLiquidity
    );
    require!(
        liquidity::has_liquidity_to_reduce_cash_with_added_collateral::<T>(
            liquidator,
            amount,
            collateral_asset,
            seize_amount,
        )?,
        Reason::InsufficientLiquidity
    );

    let (borrower_repay_principal, _borrower_supply_principal) =
        repay_and_supply_principal(liquidator_cash_principal, principal)?;
    let (_liquidator_withdraw_principal, liquidator_borrow_principal) =
        withdraw_and_borrow_principal(borrower_cash_principal, principal)?;
    let (borrower_collateral_withdraw_amount, _borrower_collateral_borrow_amount) =
        withdraw_and_borrow_amount(borrower_collateral_asset, seize_amount)?;
    let (liquidator_collateral_repay_amount, liquidator_collateral_supply_amount) =
        repay_and_supply_amount(liquidator_collateral_asset, seize_amount)?;

    let borrower_cash_principal_new = borrower_cash_principal.add_amount(principal)?;
    let liquidator_cash_principal_new = liquidator_cash_principal.sub_amount(principal)?;
    let borrower_collateral_asset_new =
        sub_amount_from_balance(borrower_collateral_asset, seize_amount)?;
    let liquidator_collateral_asset_new =
        add_amount_to_balance(liquidator_collateral_asset, seize_amount)?;

    let total_cash_principal_new = sub_principal_amounts(
        add_principal_amounts(TotalCashPrincipal::get(), liquidator_borrow_principal)?,
        borrower_repay_principal,
        Reason::RepayTooMuch,
    )?;
    let total_collateral_supply_new = sub_amount_from_raw(
        add_amount_to_raw(
            TotalSupplyAssets::get(collateral_asset.asset),
            liquidator_collateral_supply_amount,
        )?,
        borrower_collateral_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_collateral_borrow_new = sub_amount_from_raw(
        TotalBorrowAssets::get(collateral_asset.asset),
        liquidator_collateral_repay_amount,
        Reason::RepayTooMuch,
    )?;

    let (borrower_cash_principal_post, borrower_collateral_last_index_post) =
        core::effect_of_asset_interest_internal(
            collateral_asset,
            borrower,
            borrower_collateral_asset,
            borrower_collateral_asset_new,
            borrower_cash_principal_new,
        )?;
    let (liquidator_cash_principal_post, liquidator_collateral_last_index_post) =
        core::effect_of_asset_interest_internal(
            collateral_asset,
            liquidator,
            liquidator_collateral_asset,
            liquidator_collateral_asset_new,
            liquidator_cash_principal_new,
        )?;

    LastIndices::insert(
        collateral_asset.asset,
        borrower,
        borrower_collateral_last_index_post,
    );
    LastIndices::insert(
        collateral_asset.asset,
        liquidator,
        liquidator_collateral_last_index_post,
    );
    CashPrincipals::insert(borrower, borrower_cash_principal_post);
    CashPrincipals::insert(liquidator, liquidator_cash_principal_post);
    TotalCashPrincipal::put(total_cash_principal_new);
    TotalSupplyAssets::insert(collateral_asset.asset, total_collateral_supply_new);
    TotalBorrowAssets::insert(collateral_asset.asset, total_collateral_borrow_new);

    core::set_asset_balance_internal::<T>(
        collateral_asset.asset,
        borrower,
        borrower_collateral_asset_new,
    );
    core::set_asset_balance_internal::<T>(
        collateral_asset.asset,
        liquidator,
        liquidator_collateral_asset_new,
    );

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

    require!(borrower != liquidator, Reason::SelfTransfer);
    require_min_tx_value!(get_value::<T>(amount)?);

    let liquidation_incentive = Factor::from_nominal("1.08"); // XXX spec first
    let liquidator_asset = AssetBalances::get(asset.asset, liquidator);
    let borrower_asset = AssetBalances::get(asset.asset, borrower);
    let liquidator_cash_principal = CashPrincipals::get(liquidator);
    let borrower_cash_principal = CashPrincipals::get(borrower);
    let seize_amount = amount
        .mul_factor(liquidation_incentive)?
        .mul_price(get_price::<T>(asset.units())?)?
        .div_price(get_price::<T>(CASH)?, CASH)?;
    let seize_principal = index.cash_principal_amount(seize_amount)?;

    require!(
        !liquidity::has_non_negative_liquidity::<T>(borrower)?,
        Reason::SufficientLiquidity
    );
    require!(
        liquidity::has_liquidity_to_reduce_asset_with_added_cash::<T>(
            liquidator,
            asset,
            amount,
            seize_amount
        )?,
        Reason::InsufficientLiquidity
    );

    let (borrower_repay_amount, _borrower_supply_amount) =
        repay_and_supply_amount(liquidator_asset, amount)?;
    let (liquidator_withdraw_amount, liquidator_borrow_amount) =
        withdraw_and_borrow_amount(borrower_asset, amount)?;
    let (borrower_collateral_withdraw_principal, _borrower_collateral_borrow_principal) =
        withdraw_and_borrow_principal(borrower_cash_principal, seize_principal)?;
    let (liquidator_collateral_repay_principal, _liquidator_collateral_supply_principal) =
        repay_and_supply_principal(
            liquidator_cash_principal,
            borrower_collateral_withdraw_principal,
        )?;

    let borrower_asset_new = add_amount_to_balance(borrower_asset, amount)?;
    let liquidator_asset_new = sub_amount_from_balance(liquidator_asset, amount)?;
    let borrower_cash_principal_new = borrower_cash_principal.sub_amount(seize_principal)?;
    let liquidator_cash_principal_new = liquidator_cash_principal.add_amount(seize_principal)?;

    let total_supply_new = sub_amount_from_raw(
        TotalSupplyAssets::get(asset.asset),
        liquidator_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_borrow_new = sub_amount_from_raw(
        add_amount_to_raw(
            TotalBorrowAssets::get(asset.asset),
            liquidator_borrow_amount,
        )?,
        borrower_repay_amount,
        Reason::RepayTooMuch,
    )?;
    let total_cash_principal_new = sub_principal_amounts(
        TotalCashPrincipal::get(),
        liquidator_collateral_repay_principal,
        Reason::RepayTooMuch,
    )?;

    let (borrower_cash_principal_post, borrower_last_index_post) =
        core::effect_of_asset_interest_internal(
            asset,
            borrower,
            borrower_asset,
            borrower_asset_new,
            borrower_cash_principal_new,
        )?;
    let (liquidator_cash_principal_post, liquidator_last_index_post) =
        core::effect_of_asset_interest_internal(
            asset,
            liquidator,
            liquidator_asset,
            liquidator_asset_new,
            liquidator_cash_principal_new,
        )?;

    LastIndices::insert(asset.asset, borrower, borrower_last_index_post);
    LastIndices::insert(asset.asset, liquidator, liquidator_last_index_post);
    CashPrincipals::insert(borrower, borrower_cash_principal_post);
    CashPrincipals::insert(liquidator, liquidator_cash_principal_post);
    TotalSupplyAssets::insert(asset.asset, total_supply_new);
    TotalBorrowAssets::insert(asset.asset, total_borrow_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    core::set_asset_balance_internal::<T>(asset.asset, borrower, borrower_asset_new);
    core::set_asset_balance_internal::<T>(asset.asset, liquidator, liquidator_asset_new);

    <Module<T>>::deposit_event(Event::LiquidateCashCollateral(
        asset.asset,
        liquidator,
        borrower,
        amount.value,
    ));

    Ok(())
}
