// Note: The substrate build requires these be re-exported.
pub use our_std::{
    cmp::{max, min},
    convert::TryFrom,
    fmt, result,
    result::Result,
};

// Import these traits so we can interact with the substrate storage modules.
use frame_support::storage::{StorageDoubleMap, StorageMap, StorageValue};

use crate::{
    chains::eth,     // XXX event ADTs?
    notices::Notice, // XXX move here, encoding to chains
    params::MIN_TX_VALUE,
    symbol::{Symbol, CASH, NIL}, // XXX NIL tmp
    types::{
        AssetAmount, AssetBalance, AssetQuantity, CashPrincipal, CashQuantity, ChainAccount,
        ChainAsset, Int, MathError, Price, Quantity, Reason, SafeMul, USDQuantity,
    },
    AssetBalances,
    CashPrincipals,
    ChainCashPrincipals,
    Config,
    GlobalCashIndex,
    Module,
    Prices,
    RawEvent::GoldieLocks, // XXX
    TotalBorrowAssets,
    TotalCashPrincipal,
    TotalSupplyAssets,
};

macro_rules! require {
    ($expr:expr, $reason:expr) => {
        if !$expr {
            return core::result::Result::Err($reason);
        }
    };
}

macro_rules! require_min_tx_value {
    ($value:expr) => {
        require!($value >= MIN_TX_VALUE, Reason::MinTxValueNotMet);
    };
}

// Public helper functions //

pub fn symbol<T: Config>(asset: ChainAsset) -> Symbol {
    // XXX lookup in storage
    //  need to initialize with decimals first
    Symbol(
        ['E', 'T', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
        18,
    )
}

pub fn price<T: Config>(symbol: Symbol) -> Price {
    match symbol {
        CASH => Price::from_nominal(CASH, "1.0"),
        _ => Price(symbol, Prices::get(symbol)),
    }
}

/// Return the USD value of the asset amount.
pub fn value<T: Config>(amount: AssetQuantity) -> Result<USDQuantity, MathError> {
    amount.mul(price::<T>(amount.symbol()))
}

// Internal helpers //

fn add_amount_to_raw(a: AssetAmount, b: AssetQuantity) -> Result<AssetAmount, MathError> {
    Ok(a.checked_add(b.value()).ok_or(MathError::Overflow)?)
}

fn add_amount_to_balance(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> Result<AssetBalance, MathError> {
    let signed = Int::try_from(amount.value()).or(Err(MathError::Overflow))?;
    Ok(balance.checked_add(signed).ok_or(MathError::Overflow)?)
}

fn add_principals(a: CashPrincipal, b: CashPrincipal) -> Result<CashPrincipal, MathError> {
    Ok(CashPrincipal(
        a.0.checked_add(b.0).ok_or(MathError::Overflow)?,
    ))
}

fn sub_amount_from_raw(
    a: AssetAmount,
    b: AssetQuantity,
    underflow: Reason,
) -> Result<AssetAmount, Reason> {
    Ok(a.checked_sub(b.value()).ok_or(underflow)?)
}

fn sub_amount_from_balance(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> Result<AssetBalance, MathError> {
    let signed = Int::try_from(amount.value()).or(Err(MathError::Overflow))?;
    Ok(balance.checked_sub(signed).ok_or(MathError::Underflow)?)
}

fn sub_principal_from_balance(
    balance: CashPrincipal,
    principal: CashPrincipal,
) -> Result<CashPrincipal, MathError> {
    let result = balance.0.checked_sub(principal.0);
    Ok(CashPrincipal(result.ok_or(MathError::Underflow)?))
}

fn sub_principals(
    a: CashPrincipal,
    b: CashPrincipal,
    underflow: Reason,
) -> Result<CashPrincipal, Reason> {
    Ok(CashPrincipal(a.0.checked_sub(b.0).ok_or(underflow)?))
}

fn neg_balance(balance: AssetBalance) -> AssetAmount {
    if balance < 0 {
        -balance as AssetAmount
    } else {
        0
    }
}

fn pos_balance(balance: AssetBalance) -> AssetAmount {
    if balance > 0 {
        balance as AssetAmount
    } else {
        0
    }
}

fn repay_and_supply_amount(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> (AssetQuantity, AssetQuantity) {
    let Quantity(symbol, raw_amount) = amount;
    let repay_amount = min(neg_balance(balance), raw_amount);
    let supply_amount = raw_amount - repay_amount;
    (
        Quantity(symbol, repay_amount),
        Quantity(symbol, supply_amount),
    )
}

fn withdraw_and_borrow_amount(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> (AssetQuantity, AssetQuantity) {
    let Quantity(symbol, raw_amount) = amount;
    let withdraw_amount = min(pos_balance(balance), raw_amount);
    let borrow_amount = raw_amount - withdraw_amount;
    (
        Quantity(symbol, withdraw_amount),
        Quantity(symbol, borrow_amount),
    )
}

fn repay_and_supply_principal(
    balance: CashPrincipal,
    principal: CashPrincipal,
) -> (CashPrincipal, CashPrincipal) {
    let repay_principal = max(min(-balance.0, principal.0), 0);
    let supply_principal = principal.0 - repay_principal;
    (
        CashPrincipal(repay_principal),
        CashPrincipal(supply_principal),
    )
}

fn withdraw_and_borrow_principal(
    balance: CashPrincipal,
    principal: CashPrincipal,
) -> (CashPrincipal, CashPrincipal) {
    let withdraw_principal = max(min(balance.0, principal.0), 0);
    let borrow_principal = principal.0 - withdraw_principal;
    (
        CashPrincipal(withdraw_principal),
        CashPrincipal(borrow_principal),
    )
}

// Protocol interface //

pub fn apply_eth_event_internal<T: Config>(event: eth::Event) -> Result<(), Reason> {
    match event.data {
        eth::EventData::Lock {
            asset,
            holder,
            amount,
        } => lock_internal::<T>(
            ChainAsset::Eth(asset),
            ChainAccount::Eth(holder),
            Quantity(symbol::<T>(ChainAsset::Eth(asset)), amount),
        ),

        eth::EventData::LockCash {
            holder,
            amount,
            .. // XXX do we want to use index?
        } => lock_cash_internal::<T>(ChainAccount::Eth(holder), Quantity(CASH, amount)),

        eth::EventData::Gov {} => {
            // XXX also handle 'remote control' trx requests (aka do)
            // XXX are these 'do' now?
            //   Decode a SCALE-encoded set of extrinsics from the event
            //   For each extrinsic, dispatch the given extrinsic as Root
            Err(Reason::NotImplemented)
        }
    }
}

pub fn lock_internal<T: Config>(
    asset: ChainAsset,
    holder: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    // require_min_tx_value!(value::<T>(amount)?);

    let holder_asset = AssetBalances::get(asset, holder);
    let (holder_repay_amount, holder_supply_amount) = repay_and_supply_amount(holder_asset, amount);

    let holder_asset_new = add_amount_to_balance(holder_asset, amount)?;
    let total_supply_new = add_amount_to_raw(TotalSupplyAssets::get(asset), holder_supply_amount)?;
    let total_borrow_new = sub_amount_from_raw(
        TotalBorrowAssets::get(asset),
        holder_repay_amount,
        Reason::RepayTooMuch,
    )?;

    AssetBalances::insert(asset, holder, holder_asset_new);
    TotalSupplyAssets::insert(asset, total_supply_new);
    TotalBorrowAssets::insert(asset, total_borrow_new);

    // XXX real events
    <Module<T>>::deposit_event(GoldieLocks(asset, holder, amount.value())); // XXX -> raw amount?
    Ok(())
}

pub fn lock_cash_internal<T: Config>(
    holder: ChainAccount,
    amount: CashQuantity,
) -> Result<(), Reason> {
    require_min_tx_value!(value::<T>(amount)?);

    let index = GlobalCashIndex::get();
    let principal = index.as_hold_principal(amount)?;
    let holder_cash_principal = CashPrincipals::get(holder);
    let (holder_repay_principal, _holder_supply_principal) =
        repay_and_supply_principal(holder_cash_principal, principal);

    let chain_id = holder.chain_id();
    let chain_cash_principal_new = sub_principals(
        ChainCashPrincipals::get(chain_id),
        principal,
        Reason::InsufficientChainCash,
    )?;
    let holder_cash_principal_new = add_principals(holder_cash_principal, principal)?;
    let total_cash_principal_new = sub_principals(
        TotalCashPrincipal::get(),
        holder_repay_principal,
        Reason::RepayTooMuch,
    )?;

    ChainCashPrincipals::insert(chain_id, chain_cash_principal_new);
    CashPrincipals::insert(holder, holder_cash_principal_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    // XXX should we return events to be deposited?
    Ok(())
}

pub fn extract_internal<T: Config>(
    asset: ChainAsset,
    holder: ChainAccount,
    recipient: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    require!(
        recipient.chain_id() == asset.chain_id(),
        Reason::ChainMismatch
    );
    require_min_tx_value!(value::<T>(amount)?);
    require!(
        has_liquidity_to_reduce_asset(holder, amount),
        Reason::InsufficientLiquidity
    );

    let holder_asset = AssetBalances::get(asset, holder);
    let (holder_withdraw_amount, holder_borrow_amount) =
        withdraw_and_borrow_amount(holder_asset, amount);

    let holder_asset_new = sub_amount_from_balance(holder_asset, amount)?;
    let total_supply_new = sub_amount_from_raw(
        TotalSupplyAssets::get(asset),
        holder_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_borrow_new = add_amount_to_raw(TotalBorrowAssets::get(asset), holder_borrow_amount)?;

    AssetBalances::insert(asset, holder, holder_asset_new);
    TotalSupplyAssets::insert(asset, total_supply_new);
    TotalBorrowAssets::insert(asset, total_borrow_new);

    // XXX
    // Add ExtractionNotice(Asset, Recipient, Amount) to NoticeQueueRecipient.Chain

    Ok(()) // XXX events?
}

pub fn extract_cash_principal_internal<T: Config>(
    holder: ChainAccount,
    recipient: ChainAccount,
    principal: CashPrincipal,
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();
    let amount = index.as_hold_amount(principal)?;

    require_min_tx_value!(value::<T>(amount)?);
    require!(
        has_liquidity_to_reduce_cash(holder, amount),
        Reason::InsufficientLiquidity
    );

    let holder_cash_principal = CashPrincipals::get(holder);
    let (_holder_withdraw_principal, holder_borrow_principal) =
        withdraw_and_borrow_principal(holder_cash_principal, principal);

    let chain_id = recipient.chain_id();
    let chain_cash_principal_new = add_principals(ChainCashPrincipals::get(chain_id), principal)?;
    let holder_cash_principal_new = sub_principal_from_balance(holder_cash_principal, principal)?;
    let total_cash_principal_new =
        add_principals(TotalCashPrincipal::get(), holder_borrow_principal)?;

    ChainCashPrincipals::insert(chain_id, chain_cash_principal_new);
    CashPrincipals::insert(holder, holder_cash_principal_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    // XXX
    // Add CashExtractionNotice(Recipient, Amount, CashIndex) to NoticeQueueRecipient.Chain

    Ok(()) // XXX events?
}

pub fn transfer_internal<T: Config>(
    _asset: ChainAsset,
    _sender: ChainAccount,
    _recipient: ChainAccount,
    _amount: AssetQuantity,
) -> Result<(), Reason> {
    // XXX
    Ok(())
}

pub fn transfer_cash_principal_internal<T: Config>(
    _asset: ChainAsset,
    _sender: ChainAccount,
    _recipient: ChainAccount,
    _principal: CashPrincipal,
) -> Result<(), Reason> {
    // XXX
    // XXX require principal amount >= 0
    Ok(())
}

pub fn liquidate_internal<T: Config>(
    _asset: ChainAsset,
    _collateral_asset: ChainAsset,
    _liquidator: ChainAccount,
    _borrower: ChainAccount,
    _amount: AssetQuantity,
) -> Result<(), Reason> {
    // XXX
    Ok(())
}

pub fn liquidate_cash_principal_internal<T: Config>(
    _collateral_asset: ChainAsset,
    _liquidator: ChainAccount,
    _borrower: ChainAccount,
    _principal: CashPrincipal,
) -> Result<(), Reason> {
    // XXX
    Ok(())
}

pub fn liquidate_cash_collateral_internal<T: Config>(
    _asset: ChainAsset,
    _liquidator: ChainAccount,
    _borrower: ChainAccount,
    _amount: AssetQuantity,
) -> Result<(), Reason> {
    // XXX
    Ok(())
}

// Liquidity Checks //

pub fn has_liquidity_to_reduce_asset(_holder: ChainAccount, _amount: AssetQuantity) -> bool {
    true // XXX
}

pub fn has_liquidity_to_reduce_cash(_holder: ChainAccount, _amount: CashQuantity) -> bool {
    true // XXX
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::USD;

    #[test]
    fn test_helpers() {
        assert_eq!(neg_balance(100), 0);
        assert_eq!(pos_balance(-100), 0);
        assert_eq!(neg_balance(-100), 100);
        assert_eq!(pos_balance(100), 100);

        let amount = Quantity(USD, 100);
        assert_eq!(
            repay_and_supply_amount(0, amount),
            (Quantity(USD, 0), amount)
        );
        assert_eq!(
            repay_and_supply_amount(-50, amount),
            (Quantity(USD, 50), Quantity(USD, 50))
        );
        assert_eq!(
            repay_and_supply_amount(-100, amount),
            (amount, Quantity(USD, 0))
        );
        assert_eq!(
            withdraw_and_borrow_amount(0, amount),
            (Quantity(USD, 0), amount)
        );
        assert_eq!(
            withdraw_and_borrow_amount(50, amount),
            (Quantity(USD, 50), Quantity(USD, 50))
        );
        assert_eq!(
            withdraw_and_borrow_amount(100, amount),
            (amount, Quantity(USD, 0))
        );

        let principal = CashPrincipal(100);
        assert_eq!(
            repay_and_supply_principal(CashPrincipal(0), principal),
            (CashPrincipal(0), principal)
        );
        assert_eq!(
            repay_and_supply_principal(CashPrincipal(-50), principal),
            (CashPrincipal(50), CashPrincipal(50))
        );
        assert_eq!(
            repay_and_supply_principal(CashPrincipal(-100), principal),
            (principal, CashPrincipal(0))
        );
        assert_eq!(
            withdraw_and_borrow_principal(CashPrincipal(0), principal),
            (CashPrincipal(0), principal)
        );
        assert_eq!(
            withdraw_and_borrow_principal(CashPrincipal(50), principal),
            (CashPrincipal(50), CashPrincipal(50))
        );
        assert_eq!(
            withdraw_and_borrow_principal(CashPrincipal(100), principal),
            (principal, CashPrincipal(0))
        );
    }
}
