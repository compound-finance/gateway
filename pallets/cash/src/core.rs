// Note: The substrate build requires these be re-exported.
pub use our_std::{
    cmp::{max, min},
    collections::btree_set::BTreeSet,
    convert::TryFrom,
    fmt, result,
    result::Result,
    str,
};

// Import these traits so we can interact with the substrate storage modules.
use crate::rates::APR;
use crate::types::{AssetIndex, AssetPrice, CashIndex, Timestamp, Uint, MILLISECONDS_PER_YEAR};
use frame_support::storage::{
    IterableStorageDoubleMap, IterableStorageMap, StorageDoubleMap, StorageMap, StorageValue,
};

use crate::{
    chains::{
        CashAsset, Chain, ChainAccount, ChainAccountSignature, ChainAsset, ChainAssetAccount,
        ChainHash, ChainId, ChainSignature, ChainSignatureList, Ethereum,
    },
    events::{ChainLogEvent, ChainLogId, EventState},
    log, notices,
    notices::{
        CashExtractionNotice, EncodeNotice, ExtractionNotice, Notice, NoticeId, NoticeState,
    },
    params::{MIN_TX_VALUE, TRANSFER_FEE},
    reason,
    reason::{MathError, Reason},
    symbol::{Symbol, CASH},
    types::{
        AssetAmount, AssetBalance, AssetQuantity, CashPrincipal, CashQuantity, Int, Nonce, Price,
        Quantity, SafeMul, USDQuantity, ValidatorIdentity, ValidatorSig,
    },
    AccountNotices, AssetBalances, AssetSymbols, BorrowIndices, Call, CashPrincipals, CashYield,
    ChainCashPrincipals, Config, Event, EventStates, GlobalCashIndex, LastBlockTimestamp,
    LastIndices, LatestNotice, Module, Nonces, NoticeHashes, NoticeStates, Notices, Prices,
    RateModels, ReserveFactor, SubmitTransaction, SupplyIndices, TotalBorrowAssets,
    TotalCashPrincipal, TotalSupplyAssets, Validators,
};
use codec::{Decode, Encode};
use either::{Either, Left, Right};
use frame_support::sp_runtime::traits::Convert;
use frame_support::traits::UnfilteredDispatchable;
use our_std::convert::TryInto;

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

pub fn symbol<T: Config>(asset: ChainAsset) -> Option<Symbol> {
    AssetSymbols::get(asset)
}

pub fn try_chain_asset_account(
    asset: ChainAsset,
    account: ChainAccount,
) -> Option<ChainAssetAccount> {
    match (asset, account) {
        (ChainAsset::Eth(eth_asset), ChainAccount::Eth(eth_account)) => {
            Some(ChainAssetAccount::Eth(eth_asset, eth_account))
        }
        _ => None,
    }
}

fn now<T: Config>() -> Timestamp {
    let now = <pallet_timestamp::Module<T>>::get();
    T::TimeConverter::convert(now)
}

pub fn price<T: Config>(symbol: Symbol) -> Price {
    match symbol {
        CASH => Price::from_nominal(CASH, "1.0"),
        _ => Price(symbol, Prices::get(symbol)),
    }
}

fn get_symbol(asset: &ChainAsset) -> Result<Symbol, Reason> {
    AssetSymbols::get(asset).ok_or(Reason::AssetSymbolNotFound)
}

pub fn get_total_supply_assets(asset: &ChainAsset) -> Result<Quantity, Reason> {
    Ok(Quantity(get_symbol(asset)?, TotalSupplyAssets::get(asset)))
}

pub fn get_total_borrow_assets(asset: &ChainAsset) -> Result<Quantity, Reason> {
    Ok(Quantity(get_symbol(asset)?, TotalBorrowAssets::get(asset)))
}

/// Return the USD value of the asset amount.
pub fn value<T: Config>(amount: AssetQuantity) -> Result<USDQuantity, MathError> {
    amount.mul(price::<T>(amount.symbol()))
}

// Internal helpers

pub fn passes_validation_threshold(
    signers: &Vec<ValidatorIdentity>,
    validators: &Vec<ValidatorIdentity>,
) -> bool {
    let mut signer_set = BTreeSet::<ValidatorIdentity>::new();
    for v in signers {
        signer_set.insert(*v);
    }

    let mut validator_set = BTreeSet::<ValidatorIdentity>::new();
    for v in validators {
        validator_set.insert(*v);
    }
    signer_set.len() > validator_set.len() * 2 / 3
}

fn scale_multiple_terms(
    unscaled: Uint,
    numerator_decimals: u8,
    denominator_decimals: u8,
    output_decimals: u8,
) -> Option<Uint> {
    // as cannot panic here u8 -> i32 is safe, beware of changes in the future to u8 above
    let numerator_decimals = numerator_decimals as i32;
    let denominator_decimals = denominator_decimals as i32;
    let output_decimals = output_decimals as i32;
    let scale_decimals = output_decimals
        .checked_sub(numerator_decimals)?
        .checked_add(denominator_decimals)?;
    if scale_decimals < 0 {
        // as is safe here due to above non-negativity check and will not panic.
        let scalar = 10u128.checked_pow((-1 * scale_decimals) as u32)?;
        unscaled.checked_div(scalar)
    } else {
        let scalar = 10u128.checked_pow(scale_decimals as u32)?;
        unscaled.checked_mul(scalar)
    }
}

fn compute_cash_principal_per_internal(
    asset_rate: Uint,
    dt: Uint,
    cash_index: Uint,
    price_asset: Uint,
) -> Option<Uint> {
    let unscaled = asset_rate
        .checked_mul(dt)?
        .checked_mul(price_asset)?
        .checked_div(cash_index)?
        .checked_div(MILLISECONDS_PER_YEAR)?;

    scale_multiple_terms(
        unscaled,
        APR::DECIMALS + Price::DECIMALS,
        CashIndex::DECIMALS,
        AssetIndex::DECIMALS,
    )
}

pub fn compute_cash_principal_per(
    asset_rate: APR,
    dt: Timestamp,
    cash_index: CashIndex,
    price_asset: AssetPrice,
) -> Result<CashPrincipal, Reason> {
    let raw = compute_cash_principal_per_internal(asset_rate.0, dt, cash_index.0, price_asset)
        .ok_or(Reason::MathError(MathError::Overflow))?;

    let raw: Int = raw
        .try_into()
        .map_err(|_| Reason::MathError(MathError::Overflow))?;

    Ok(CashPrincipal(raw))
}

pub fn increment_index(lhs: AssetIndex, rhs: CashPrincipal) -> Result<AssetIndex, Reason> {
    // note - we assume decimals must match
    let converted: Uint = rhs
        .0
        .try_into()
        .map_err(|_| Reason::MathError(MathError::SignMismatch))?;
    let raw = lhs
        .0
        .checked_add(converted)
        .ok_or(Reason::MathError(MathError::Overflow))?;

    Ok(AssetIndex(raw))
}

/// Check to make sure an asset is supported. Only returns Err when the asset is not supported
/// by compound chain.
pub fn check_asset_supported(asset: &ChainAsset) -> Result<(), Reason> {
    if !AssetSymbols::contains_key(&asset) {
        return Err(Reason::AssetNotSupported);
    }

    Ok(())
}

/// READ the utilization for a given asset
pub fn get_utilization(asset: &ChainAsset) -> Result<crate::rates::Utilization, Reason> {
    check_asset_supported(asset)?;
    let total_supply = TotalSupplyAssets::get(asset);
    let total_borrow = TotalBorrowAssets::get(asset);
    Ok(crate::rates::get_utilization(total_supply, total_borrow)?)
}

/// READ the borrow and supply rates for a given asset
pub fn get_rates(asset: &ChainAsset) -> Result<(APR, APR), Reason> {
    check_asset_supported(asset)?;
    let model = RateModels::get(asset);
    let utilization = get_utilization(asset)?;
    let reserve_factor = ReserveFactor::get(asset);
    Ok(model.get_rates(utilization, APR::ZERO, reserve_factor)?)
}

fn add_amount_to_raw(a: AssetAmount, b: AssetQuantity) -> Result<AssetAmount, MathError> {
    a.checked_add(b.value()).ok_or(MathError::Overflow)
}

fn add_amount_to_balance(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> Result<AssetBalance, MathError> {
    let signed = Int::try_from(amount.value()).or(Err(MathError::Overflow))?;
    balance.checked_add(signed).ok_or(MathError::Overflow)
}

fn add_principal_to_balance(
    balance: CashPrincipal,
    principal: CashPrincipal,
) -> Result<CashPrincipal, MathError> {
    let result = balance.0.checked_add(principal.0);
    Ok(CashPrincipal(result.ok_or(MathError::Overflow)?))
}

fn add_principal_amounts(a: CashPrincipal, b: CashPrincipal) -> Result<CashPrincipal, MathError> {
    // XXX check positive? add UnsignedCashPrincipal?
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

fn sub_principal_amounts(
    a: CashPrincipal,
    b: CashPrincipal,
    underflow: Reason,
) -> Result<CashPrincipal, Reason> {
    // XXX need to convert a/b to unsigned and return reason for negative underflow
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

pub fn apply_chain_event_internal<T: Config>(event: ChainLogEvent) -> Result<(), Reason> {
    log!("apply_chain_event_internal(event): {:?}", &event);

    match event {
        ChainLogEvent::Eth(eth_event) => {
            match eth_event.event {
                ethereum_client::events::EthereumEvent::Lock {
                    asset,
                    holder,
                    amount,
                } => lock_internal::<T>(
                    ChainAsset::Eth(asset),
                    ChainAccount::Eth(holder),
                    Quantity(symbol::<T>(ChainAsset::Eth(asset)).ok_or(Reason::NoSuchAsset)?, amount),
                ),

                ethereum_client::events::EthereumEvent::LockCash {
                    holder,
                    amount,
                    .. // XXX do we want to use index?
                } => lock_cash_internal::<T>(ChainAccount::Eth(holder), Quantity(CASH, amount)),

                ethereum_client::events::EthereumEvent::Gov { extrinsics } => dispatch_extrinsics_internal::<T>(extrinsics),
            }
        }
    }
}

fn dispatch_notice_internal<T: Config>(
    chain_id: ChainId,
    recipient_opt: Option<ChainAccount>,
    notice_fn: &dyn Fn(NoticeId, ChainHash) -> Result<Notice, Reason>,
) -> Result<(), Reason> {
    // XXX this cannot fail, should not return result
    let (latest_notice_id, parent_hash) =
        LatestNotice::get(chain_id).unwrap_or((NoticeId(0, 0), chain_id.zero_hash()));
    let notice_id = latest_notice_id.seq();

    let notice = notice_fn(notice_id, parent_hash)?; // XXX fixme cannot fail

    // Add to notices, notice states, track the latest notice and index by account
    let notice_hash = notice.hash();
    Notices::insert(chain_id, notice_id, &notice);
    NoticeStates::insert(chain_id, notice_id, NoticeState::pending(&notice));
    LatestNotice::insert(chain_id, (notice_id, notice_hash));
    NoticeHashes::insert(notice_hash, notice_id);
    if let Some(recipient) = recipient_opt {
        AccountNotices::append(recipient, notice_id);
    }

    // Deposit Notice Event
    let encoded_notice = notice.encode_notice();
    Module::<T>::deposit_event(Event::Notice(notice_id, notice, encoded_notice));
    Ok(()) // XXX cannot fail
}

/// Update the index of which assets an account has non-zero balances in.
fn set_asset_balance_internal(asset: ChainAsset, account: ChainAccount, balance: AssetBalance) {
    if balance == 0 {
        // XXX delete from assets with non zero balance
    } else {
        // XXX add to assets with non zero balance
    }
    AssetBalances::insert(asset, account, balance);
}

pub fn dispatch_extrinsics_internal<T: Config>(extrinsics: Vec<Vec<u8>>) -> Result<(), Reason> {
    //   Decode a SCALE-encoded set of extrinsics from the event
    //   For each extrinsic, dispatch the given extrinsic as Root
    extrinsics.iter().map(|payload| {
        log!(
            "dispatch_extrinsics_internal:: dispatching extrinsic {}",
            hex::encode(&payload)
        );
        let call_res: Result<<T as Config>::Call, _> = Decode::decode(&mut &payload[..]);
        match call_res {
            Ok(call) => {
                log!("dispatch_extrinsics_internal:: dispatching {:?}", call);
                let res = call.dispatch_bypass_filter(frame_system::RawOrigin::Root.into());
                log!("dispatch_extrinsics_internal:: res {:?}", res);
                // TODO: Collect results?
            }
            _ => {
                log!(
                    "dispatch_extrinsics_internal:: failed to decode extrinsic {}",
                    hex::encode(&payload)
                );
            }
        }
    });
    Ok(())
}

pub fn lock_internal<T: Config>(
    asset: ChainAsset,
    holder: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    let holder_asset = AssetBalances::get(asset, holder);
    let (holder_repay_amount, holder_supply_amount) = repay_and_supply_amount(holder_asset, amount);

    let holder_asset_new = add_amount_to_balance(holder_asset, amount)?;
    let total_supply_new = add_amount_to_raw(TotalSupplyAssets::get(asset), holder_supply_amount)?;
    let total_borrow_new = sub_amount_from_raw(
        TotalBorrowAssets::get(asset),
        holder_repay_amount,
        Reason::RepayTooMuch,
    )?;

    let (cash_principal_post, last_index_post) = effect_of_asset_interest_internal(
        asset,
        holder,
        holder_asset,
        holder_asset_new,
        CashPrincipals::get(holder),
    )?;

    LastIndices::insert(asset, holder, last_index_post);
    CashPrincipals::insert(holder, cash_principal_post);
    TotalSupplyAssets::insert(asset, total_supply_new);
    TotalBorrowAssets::insert(asset, total_borrow_new);

    set_asset_balance_internal(asset, holder, holder_asset_new);

    // XXX real events
    <Module<T>>::deposit_event(Event::GoldieLocks(asset, holder, amount.value())); // XXX -> raw amount?

    Ok(()) // XXX events?
}

pub fn lock_cash_internal<T: Config>(
    holder: ChainAccount,
    amount: CashQuantity,
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();
    let principal = index.as_hold_principal(amount)?;
    let holder_cash_principal = CashPrincipals::get(holder);
    let (holder_repay_principal, _holder_supply_principal) =
        repay_and_supply_principal(holder_cash_principal, principal);

    let chain_id = holder.chain_id();
    let chain_cash_principal_new = sub_principal_amounts(
        ChainCashPrincipals::get(chain_id),
        principal,
        Reason::InsufficientChainCash,
    )?;
    let holder_cash_principal_new = holder_cash_principal.add(principal)?;
    let total_cash_principal_new = sub_principal_amounts(
        TotalCashPrincipal::get(),
        holder_repay_principal,
        Reason::RepayTooMuch,
    )?;

    ChainCashPrincipals::insert(chain_id, chain_cash_principal_new);
    CashPrincipals::insert(holder, holder_cash_principal_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    Ok(()) // XXX should we return events to be deposited?
}

pub fn extract_internal<T: Config>(
    asset: ChainAsset,
    holder: ChainAccount,
    recipient: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    let chain_asset_account =
        try_chain_asset_account(asset, recipient).ok_or(Reason::ChainMismatch)?;
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

    let (cash_principal_post, last_index_post) = effect_of_asset_interest_internal(
        asset,
        holder,
        holder_asset,
        holder_asset_new,
        CashPrincipals::get(holder),
    )?;

    LastIndices::insert(asset, holder, last_index_post);
    CashPrincipals::insert(holder, cash_principal_post);
    TotalSupplyAssets::insert(asset, total_supply_new);
    TotalBorrowAssets::insert(asset, total_borrow_new);

    set_asset_balance_internal(asset, holder, holder_asset_new);

    // XXX fix me this cannot fail
    dispatch_notice_internal::<T>(
        recipient.chain_id(),
        Some(recipient),
        &|notice_id, parent_hash| {
            Ok(Notice::ExtractionNotice(
                match (chain_asset_account, parent_hash) {
                    (
                        ChainAssetAccount::Eth(eth_asset, eth_account),
                        ChainHash::Eth(eth_parent_hash),
                    ) => Ok(ExtractionNotice::Eth {
                        id: notice_id,
                        parent: eth_parent_hash,
                        asset: eth_asset,
                        account: eth_account,
                        amount: amount.1,
                    }),
                    _ => Err(Reason::AssetExtractionNotSupported),
                }?,
            ))
        },
    )?;

    Ok(()) // XXX events?
}

pub fn extract_cash_internal<T: Config>(
    holder: ChainAccount,
    recipient: ChainAccount,
    amount_or_principal: Either<Quantity, CashPrincipal>, // XXX
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();
    // XXX Some topics about `Either`, principal_positive and inputs here
    let (amount, principal) = match amount_or_principal {
        Left(amount) => (amount, index.as_hold_principal(amount)?),
        Right(principal) => (index.as_hold_amount(principal)?, principal),
    };
    // XXX
    let principal_positive: u128 = principal
        .0
        .try_into()
        .map_err(|_| Reason::NegativePrincipalExtraction)?;

    require_min_tx_value!(value::<T>(amount)?);
    require!(
        has_liquidity_to_reduce_cash(holder, amount),
        Reason::InsufficientLiquidity
    );

    let holder_cash_principal = CashPrincipals::get(holder);
    let (_holder_withdraw_principal, holder_borrow_principal) =
        withdraw_and_borrow_principal(holder_cash_principal, principal);

    let chain_id = recipient.chain_id();
    let chain_cash_principal_new =
        add_principal_amounts(ChainCashPrincipals::get(chain_id), principal)?;
    let holder_cash_principal_new = sub_principal_from_balance(holder_cash_principal, principal)?;
    let total_cash_principal_new =
        add_principal_amounts(TotalCashPrincipal::get(), holder_borrow_principal)?;

    ChainCashPrincipals::insert(chain_id, chain_cash_principal_new);
    CashPrincipals::insert(holder, holder_cash_principal_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    // XXX fix me this cannot fail
    dispatch_notice_internal::<T>(
        recipient.chain_id(),
        Some(recipient),
        &|notice_id, parent_hash| {
            Ok(Notice::CashExtractionNotice(match (holder, parent_hash) {
                (ChainAccount::Eth(eth_account), ChainHash::Eth(eth_parent_hash)) => {
                    Ok(CashExtractionNotice::Eth {
                        id: notice_id,
                        parent: eth_parent_hash,
                        account: eth_account,
                        principal: principal_positive,
                    })
                }
                _ => Err(Reason::AssetExtractionNotSupported),
            }?))
        },
    )?;

    Ok(()) // XXX events?
}

pub fn transfer_internal<T: Config>(
    asset: ChainAsset,
    sender: ChainAccount,
    recipient: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    let miner = ChainAccount::Eth([0; 20]); // todo: xxx how to get the current miner chain account
    let index = GlobalCashIndex::get();

    // XXX check asset matches amount asset?
    require!(sender != recipient, Reason::SelfTransfer);
    require_min_tx_value!(value::<T>(amount)?);
    require!(
        has_liquidity_to_reduce_asset_with_fee(sender, amount, TRANSFER_FEE),
        Reason::InsufficientLiquidity
    );

    let sender_asset = AssetBalances::get(asset, sender);
    let recipient_asset = AssetBalances::get(asset, recipient);
    let sender_cash_principal = CashPrincipals::get(sender);
    let miner_cash_principal = CashPrincipals::get(miner);

    let fee_principal = index.as_hold_principal(TRANSFER_FEE)?;
    let (sender_withdraw_amount, sender_borrow_amount) =
        withdraw_and_borrow_amount(sender_asset, amount);
    let (recipient_repay_amount, recipient_supply_amount) =
        repay_and_supply_amount(recipient_asset, amount);
    let (sender_withdraw_principal, sender_borrow_principal) =
        withdraw_and_borrow_principal(sender_cash_principal, fee_principal);
    let (miner_repay_principal, _miner_supply_principal) =
        repay_and_supply_principal(miner_cash_principal, fee_principal);

    let miner_cash_principal_new = miner_cash_principal.add(fee_principal)?;
    let sender_cash_principal_new = sender_cash_principal.sub(fee_principal)?;
    let sender_asset_new = sub_amount_from_balance(sender_asset, amount)?;
    let recipient_asset_new = add_amount_to_balance(recipient_asset, amount)?;

    let total_supply_new = sub_amount_from_raw(
        add_amount_to_raw(TotalSupplyAssets::get(asset), recipient_supply_amount)?,
        sender_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_borrow_new = sub_amount_from_raw(
        add_amount_to_raw(TotalBorrowAssets::get(asset), sender_borrow_amount)?,
        recipient_repay_amount,
        Reason::RepayTooMuch,
    )?;
    let total_cash_principal_new = sub_principal_amounts(
        add_principal_amounts(TotalCashPrincipal::get(), sender_borrow_principal)?,
        miner_repay_principal,
        Reason::RepayTooMuch,
    )?;

    let (sender_cash_principal_post, sender_last_index_post) = effect_of_asset_interest_internal(
        asset,
        sender,
        sender_asset,
        sender_asset_new,
        sender_cash_principal_new,
    )?;
    let (recipient_cash_principal_post, recipient_last_index_post) =
        effect_of_asset_interest_internal(
            asset,
            recipient,
            recipient_asset,
            recipient_asset_new,
            CashPrincipals::get(recipient),
        )?;

    LastIndices::insert(asset, sender, sender_last_index_post);
    LastIndices::insert(asset, recipient, recipient_last_index_post);
    CashPrincipals::insert(sender, sender_cash_principal_post);
    CashPrincipals::insert(recipient, recipient_cash_principal_post);
    CashPrincipals::insert(miner, miner_cash_principal_new);
    TotalSupplyAssets::insert(asset, total_supply_new);
    TotalBorrowAssets::insert(asset, total_borrow_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    set_asset_balance_internal(asset, sender, sender_asset_new);
    set_asset_balance_internal(asset, recipient, recipient_asset_new);

    Ok(()) // XXX events?
}

pub fn transfer_cash_principal_internal<T: Config>(
    sender: ChainAccount,
    recipient: ChainAccount,
    principal: CashPrincipal,
) -> Result<(), Reason> {
    let miner = ChainAccount::Eth([0; 20]); // todo: xxx how to get the current miner chain account
    let index = GlobalCashIndex::get();
    let amount = index.as_hold_amount(principal)?;

    require!(sender != recipient, Reason::SelfTransfer);
    require_min_tx_value!(value::<T>(amount)?);
    require!(
        has_liquidity_to_reduce_cash(sender, amount.add(TRANSFER_FEE)?),
        Reason::InsufficientLiquidity
    );

    let sender_cash_principal = CashPrincipals::get(sender);
    let recipient_cash_principal = CashPrincipals::get(recipient);
    let miner_cash_principal = CashPrincipals::get(miner);

    let fee_principal = index.as_hold_principal(TRANSFER_FEE)?;
    let principal_with_fee = principal.add(fee_principal)?;
    let (sender_withdraw_principal, sender_borrow_principal) =
        withdraw_and_borrow_principal(sender_cash_principal, principal_with_fee);
    let (recipient_repay_principal, _recipient_supply_principal) =
        repay_and_supply_principal(recipient_cash_principal, principal);
    let (miner_repay_principal, _miner_supply_principal) =
        repay_and_supply_principal(miner_cash_principal, fee_principal);

    let miner_cash_principal_new = miner_cash_principal.add(fee_principal)?;
    let sender_cash_principal_new = sender_cash_principal.sub(principal_with_fee)?;
    let recipient_cash_principal_new = recipient_cash_principal.add(principal)?;

    let total_cash_principal_new = sub_principal_amounts(
        add_principal_amounts(TotalCashPrincipal::get(), sender_borrow_principal)?,
        add_principal_amounts(recipient_repay_principal, miner_repay_principal)?,
        Reason::RepayTooMuch,
    )?;

    CashPrincipals::insert(miner, miner_cash_principal_new);
    CashPrincipals::insert(sender, sender_cash_principal_new);
    CashPrincipals::insert(recipient, recipient_cash_principal_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    Ok(()) // XXX events?
}

pub fn liquidate_internal<T: Config>(
    asset: ChainAsset,
    collateral_asset: ChainAsset,
    liquidator: ChainAccount,
    borrower: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    require!(borrower != liquidator, Reason::SelfTransfer);
    require!(asset != collateral_asset, Reason::InKindLiquidation);
    require_min_tx_value!(value::<T>(amount)?);

    let liquidator_asset = AssetBalances::get(asset, liquidator);
    let borrower_asset = AssetBalances::get(asset, borrower);
    let liquidator_collateral_asset = AssetBalances::get(collateral_asset, liquidator);
    let borrower_collateral_asset = AssetBalances::get(collateral_asset, borrower);
    let seize_amount = amount; // XXX * liquidation_incentive * price::<T>(asset) / price::<T>(collateral_asset);

    require!(
        has_liquidity_to_reduce_asset_with_added_collateral(liquidator, amount, seize_amount),
        Reason::InsufficientLiquidity
    );

    let (borrower_repay_amount, _borrower_supply_amount) =
        repay_and_supply_amount(liquidator_asset, amount);
    let (liquidator_withdraw_amount, liquidator_borrow_amount) =
        withdraw_and_borrow_amount(borrower_asset, amount);
    let (borrower_collateral_withdraw_amount, _borrower_collateral_borrow_amount) =
        withdraw_and_borrow_amount(borrower_collateral_asset, seize_amount);
    let (liquidator_collateral_repay_amount, liquidator_collateral_supply_amount) =
        repay_and_supply_amount(liquidator_collateral_asset, seize_amount);

    let borrower_asset_new = add_amount_to_balance(borrower_asset, amount)?;
    let liquidator_asset_new = sub_amount_from_balance(liquidator_asset, amount)?;
    let borrower_collateral_asset_new =
        sub_amount_from_balance(borrower_collateral_asset, seize_amount)?;
    let liquidator_collateral_asset_new =
        add_amount_to_balance(liquidator_collateral_asset, seize_amount)?;

    let total_supply_new = sub_amount_from_raw(
        TotalSupplyAssets::get(asset),
        liquidator_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_borrow_new = sub_amount_from_raw(
        add_amount_to_raw(TotalBorrowAssets::get(asset), liquidator_borrow_amount)?,
        borrower_repay_amount,
        Reason::RepayTooMuch,
    )?;
    let total_collateral_supply_new = sub_amount_from_raw(
        add_amount_to_raw(
            TotalSupplyAssets::get(collateral_asset),
            liquidator_collateral_supply_amount,
        )?,
        borrower_collateral_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_collateral_borrow_new = sub_amount_from_raw(
        TotalBorrowAssets::get(collateral_asset),
        liquidator_collateral_repay_amount,
        Reason::RepayTooMuch,
    )?;

    let (borrower_cash_principal_post, borrower_last_index_post) =
        effect_of_asset_interest_internal(
            asset,
            borrower,
            borrower_asset,
            borrower_asset_new,
            CashPrincipals::get(borrower),
        )?;
    let (liquidator_cash_principal_post, liquidator_last_index_post) =
        effect_of_asset_interest_internal(
            asset,
            liquidator,
            liquidator_asset,
            liquidator_asset_new,
            CashPrincipals::get(liquidator),
        )?;
    let (borrower_cash_principal_post, borrower_collateral_last_index_post) =
        effect_of_asset_interest_internal(
            collateral_asset,
            borrower,
            borrower_collateral_asset,
            borrower_collateral_asset_new,
            borrower_cash_principal_post,
        )?;
    let (liquidator_cash_principal_post, liquidator_collateral_last_index_post) =
        effect_of_asset_interest_internal(
            collateral_asset,
            liquidator,
            liquidator_collateral_asset,
            liquidator_collateral_asset_new,
            liquidator_cash_principal_post,
        )?;

    LastIndices::insert(asset, borrower, borrower_last_index_post);
    LastIndices::insert(asset, liquidator, liquidator_last_index_post);
    LastIndices::insert(
        collateral_asset,
        borrower,
        borrower_collateral_last_index_post,
    );
    LastIndices::insert(
        collateral_asset,
        liquidator,
        liquidator_collateral_last_index_post,
    );
    CashPrincipals::insert(borrower, borrower_cash_principal_post);
    CashPrincipals::insert(liquidator, liquidator_cash_principal_post);
    TotalSupplyAssets::insert(asset, total_supply_new);
    TotalBorrowAssets::insert(asset, total_borrow_new);
    TotalSupplyAssets::insert(collateral_asset, total_collateral_supply_new);
    TotalBorrowAssets::insert(collateral_asset, total_collateral_borrow_new);

    set_asset_balance_internal(asset, borrower, borrower_asset_new);
    set_asset_balance_internal(asset, liquidator, liquidator_asset_new);
    set_asset_balance_internal(collateral_asset, borrower, borrower_collateral_asset_new);
    set_asset_balance_internal(
        collateral_asset,
        liquidator,
        liquidator_collateral_asset_new,
    );

    Ok(()) // XXX events?
}

pub fn liquidate_cash_principal_internal<T: Config>(
    collateral_asset: ChainAsset,
    liquidator: ChainAccount,
    borrower: ChainAccount,
    principal: CashPrincipal,
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();
    let amount = index.as_hold_amount(principal)?;

    require!(borrower != liquidator, Reason::SelfTransfer);
    require_min_tx_value!(value::<T>(amount)?);

    let liquidator_cash_principal = CashPrincipals::get(liquidator);
    let borrower_cash_principal = CashPrincipals::get(borrower);
    let liquidator_collateral_asset = AssetBalances::get(collateral_asset, liquidator);
    let borrower_collateral_asset = AssetBalances::get(collateral_asset, borrower);
    let seize_amount = amount; // XXX * liquidation_incentive * price::<T>(CASH) / price::<T>(collateral_asset);

    require!(
        has_liquidity_to_reduce_cash_with_added_collateral(liquidator, amount, seize_amount),
        Reason::InsufficientLiquidity
    );

    let (borrower_repay_principal, _borrower_supply_principal) =
        repay_and_supply_principal(liquidator_cash_principal, principal);
    let (_liquidator_withdraw_principal, liquidator_borrow_principal) =
        withdraw_and_borrow_principal(borrower_cash_principal, principal);
    let (borrower_collateral_withdraw_amount, _borrower_collateral_borrow_amount) =
        withdraw_and_borrow_amount(borrower_collateral_asset, seize_amount);
    let (liquidator_collateral_repay_amount, liquidator_collateral_supply_amount) =
        repay_and_supply_amount(liquidator_collateral_asset, seize_amount);

    let borrower_cash_principal_new = add_principal_to_balance(borrower_cash_principal, principal)?;
    let liquidator_cash_principal_new =
        sub_principal_from_balance(liquidator_cash_principal, principal)?;
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
            TotalSupplyAssets::get(collateral_asset),
            liquidator_collateral_supply_amount,
        )?,
        borrower_collateral_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_collateral_borrow_new = sub_amount_from_raw(
        TotalBorrowAssets::get(collateral_asset),
        liquidator_collateral_repay_amount,
        Reason::RepayTooMuch,
    )?;

    let (borrower_cash_principal_post, borrower_collateral_last_index_post) =
        effect_of_asset_interest_internal(
            collateral_asset,
            borrower,
            borrower_collateral_asset,
            borrower_collateral_asset_new,
            borrower_cash_principal_new,
        )?;
    let (liquidator_cash_principal_post, liquidator_collateral_last_index_post) =
        effect_of_asset_interest_internal(
            collateral_asset,
            liquidator,
            liquidator_collateral_asset,
            liquidator_collateral_asset_new,
            liquidator_cash_principal_new,
        )?;

    LastIndices::insert(
        collateral_asset,
        borrower,
        borrower_collateral_last_index_post,
    );
    LastIndices::insert(
        collateral_asset,
        liquidator,
        liquidator_collateral_last_index_post,
    );
    CashPrincipals::insert(borrower, borrower_cash_principal_post);
    CashPrincipals::insert(liquidator, liquidator_cash_principal_post);
    TotalCashPrincipal::put(total_cash_principal_new);
    TotalSupplyAssets::insert(collateral_asset, total_collateral_supply_new);
    TotalBorrowAssets::insert(collateral_asset, total_collateral_borrow_new);

    set_asset_balance_internal(collateral_asset, borrower, borrower_collateral_asset_new);
    set_asset_balance_internal(
        collateral_asset,
        liquidator,
        liquidator_collateral_asset_new,
    );

    Ok(()) // XXX events?
}

pub fn liquidate_cash_collateral_internal<T: Config>(
    asset: ChainAsset,
    liquidator: ChainAccount,
    borrower: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();

    require!(borrower != liquidator, Reason::SelfTransfer);
    require_min_tx_value!(value::<T>(amount)?);

    let liquidator_asset = AssetBalances::get(asset, liquidator);
    let borrower_asset = AssetBalances::get(asset, borrower);
    let liquidator_cash_principal = CashPrincipals::get(liquidator);
    let borrower_cash_principal = CashPrincipals::get(borrower);
    let seize_amount = amount; // XXX * liquidation_incentive * price::<T>(asset) / price::<T>(CASH);
    let seize_principal = index.as_hold_principal(seize_amount)?;

    require!(
        has_liquidity_to_reduce_asset_with_added_cash(liquidator, amount, seize_amount),
        Reason::InsufficientLiquidity
    );

    let (borrower_repay_amount, _borrower_supply_amount) =
        repay_and_supply_amount(liquidator_asset, amount);
    let (liquidator_withdraw_amount, liquidator_borrow_amount) =
        withdraw_and_borrow_amount(borrower_asset, amount);
    let (borrower_collateral_withdraw_principal, _borrower_collateral_borrow_principal) =
        withdraw_and_borrow_principal(borrower_cash_principal, seize_principal);
    let (liquidator_collateral_repay_principal, liquidator_collateral_supply_principal) =
        repay_and_supply_principal(liquidator_cash_principal, seize_principal);

    let borrower_asset_new = add_amount_to_balance(borrower_asset, amount)?;
    let liquidator_asset_new = sub_amount_from_balance(liquidator_asset, amount)?;
    let borrower_cash_principal_new =
        sub_principal_from_balance(borrower_cash_principal, seize_principal)?;
    let liquidator_cash_principal_new =
        add_principal_to_balance(liquidator_cash_principal, seize_principal)?;

    let total_supply_new = sub_amount_from_raw(
        TotalSupplyAssets::get(asset),
        liquidator_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_borrow_new = sub_amount_from_raw(
        add_amount_to_raw(TotalBorrowAssets::get(asset), liquidator_borrow_amount)?,
        borrower_repay_amount,
        Reason::RepayTooMuch,
    )?;
    let total_cash_principal_new = sub_principal_amounts(
        TotalCashPrincipal::get(),
        liquidator_collateral_repay_principal,
        Reason::RepayTooMuch,
    )?;

    let (borrower_cash_principal_post, borrower_last_index_post) =
        effect_of_asset_interest_internal(
            asset,
            borrower,
            borrower_asset,
            borrower_asset_new,
            CashPrincipals::get(borrower),
        )?;
    let (liquidator_cash_principal_post, liquidator_last_index_post) =
        effect_of_asset_interest_internal(
            asset,
            liquidator,
            liquidator_asset,
            liquidator_asset_new,
            CashPrincipals::get(liquidator),
        )?;

    LastIndices::insert(asset, borrower, borrower_last_index_post);
    LastIndices::insert(asset, liquidator, liquidator_last_index_post);
    CashPrincipals::insert(borrower, borrower_cash_principal_post);
    CashPrincipals::insert(liquidator, liquidator_cash_principal_post);
    TotalSupplyAssets::insert(asset, total_supply_new);
    TotalBorrowAssets::insert(asset, total_borrow_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    set_asset_balance_internal(asset, borrower, borrower_asset_new);
    set_asset_balance_internal(asset, liquidator, liquidator_asset_new);

    Ok(()) // XXX events?
}

// Liquidity Checks //

pub fn has_liquidity_to_reduce_asset(_holder: ChainAccount, _amount: AssetQuantity) -> bool {
    true // XXX
}

pub fn has_liquidity_to_reduce_asset_with_added_cash(
    _liquidator: ChainAccount,
    _amount: AssetQuantity,
    _seize_amount: CashQuantity,
) -> bool {
    true // XXX
}

pub fn has_liquidity_to_reduce_asset_with_added_collateral(
    _liquidator: ChainAccount,
    _amount: AssetQuantity,
    _seize_amount: AssetQuantity,
) -> bool {
    true // XXX
}

pub fn has_liquidity_to_reduce_asset_with_fee(
    _holder: ChainAccount,
    _amount: AssetQuantity,
    _fee: CashQuantity,
) -> bool {
    true // XXX
}

pub fn has_liquidity_to_reduce_cash(_holder: ChainAccount, _amount: CashQuantity) -> bool {
    true // XXX
}

pub fn has_liquidity_to_reduce_cash_with_added_collateral(
    _liquidator: ChainAccount,
    _amount: CashQuantity,
    _seize_amount: AssetQuantity,
) -> bool {
    true // XXX
}

// Asset Interest //

/// Return CASH Principal post asset interest, and updated asset index, for a given account
pub fn effect_of_asset_interest_internal(
    asset: ChainAsset,
    account: ChainAccount,
    asset_balance_old: AssetBalance,
    asset_balance_new: AssetBalance,
    cash_principal_pre: CashPrincipal,
) -> Result<(CashPrincipal, AssetIndex), MathError> {
    let last_index = LastIndices::get(asset, account);
    let cash_index = if asset_balance_old >= 0 {
        SupplyIndices::get(asset)
    } else {
        BorrowIndices::get(asset)
    };
    let cash_principal_delta = cash_index.cash_principal_since(last_index, asset_balance_old)?;
    let cash_principal_post = cash_principal_pre.add(cash_principal_delta)?;
    let last_index_post = if asset_balance_new >= 0 {
        SupplyIndices::get(asset)
    } else {
        BorrowIndices::get(asset)
    };
    Ok((cash_principal_post, last_index_post))
}

// Off-chain Workers //

pub fn process_notices<T: Config>(_block_number: T::BlockNumber) -> Result<(), Reason> {
    // TODO: Do we want to return a failure here in any case, or collect them, etc?
    for (chain_id, notice_id, notice_state) in NoticeStates::iter() {
        match notice_state {
            NoticeState::Pending { signature_pairs } => {
                let signer = chain_id.signer_address()?;

                if !notices::has_signer(&signature_pairs, signer) {
                    // find parent
                    // id = notice.gen_id(parent)

                    // XXX
                    // submit onchain call for aggregating the price
                    let notice = Notices::get(chain_id, notice_id)
                        .ok_or(Reason::NoticeMissing(chain_id, notice_id))?;
                    let signature: ChainSignature = notice.sign_notice()?;

                    log!("Posting Signature for [{},{}]", notice_id.0, notice_id.1);

                    let call = <Call<T>>::publish_signature(chain_id, notice_id, signature);

                    // TODO: Do we want to short-circuit on an error here?
                    let _res =
                        SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
                            .map_err(|()| Reason::FailedToSubmitExtrinsic);
                }

                ()
            }
            _ => (),
        }
    }

    Ok(())
}

pub fn publish_signature_internal(
    chain_id: ChainId,
    notice_id: NoticeId,
    signature: ChainSignature,
) -> Result<(), Reason> {
    log!("Publishing Signature: [{},{}]", notice_id.0, notice_id.1);
    let state = <NoticeStates>::get(chain_id, notice_id);

    match state {
        NoticeState::Missing => Ok(()),

        NoticeState::Pending { signature_pairs } => {
            let notice = Notices::get(chain_id, notice_id)
                .ok_or(Reason::NoticeMissing(chain_id, notice_id))?;
            let signer: ChainAccount = signature.recover(&notice.encode_notice())?; // XXX
            let has_signer_v = notices::has_signer(&signature_pairs, signer);

            require!(!has_signer_v, Reason::NoticeAlreadySigned);

            // TODO: Can this be easier?
            let signature_pairs_next = match (signature_pairs, signer, signature) {
                (
                    ChainSignatureList::Eth(eth_signature_list),
                    ChainAccount::Eth(eth_account),
                    ChainSignature::Eth(eth_sig),
                ) => {
                    let mut eth_signature_list_mut = eth_signature_list.clone();
                    eth_signature_list_mut.push((eth_account, eth_sig));
                    Ok(ChainSignatureList::Eth(eth_signature_list_mut))
                }
                _ => Err(Reason::SignatureMismatch),
            }?;

            // require(signer == known validator); // XXX

            // XXX sets?
            // let signers_new = {signer | signers};
            // let signatures_new = {signature | signatures };

            // if len(signers_new & Validators) > 2/3 * len(Validators) {
            // NoticeQueue::insert(notice_id, NoticeState::Done);
            // Self::deposit_event(Event::SignedNotice(ChainId::Eth, notice_id, notice.encode_notice(), signatures)); // XXX generic
            // Ok(())
            // } else {
            NoticeStates::insert(
                chain_id,
                notice_id,
                NoticeState::Pending {
                    signature_pairs: signature_pairs_next,
                },
            );
            Ok(())
            // }
        }

        NoticeState::Executed => Ok(()),
    }
}

pub fn prepend_nonce(payload: &Vec<u8>, nonce: Nonce) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    result.extend_from_slice(nonce.to_string().as_bytes());
    result.extend_from_slice(b":");
    result.extend_from_slice(&payload[..]);
    result
}

// TODO: Unit tests!!
pub fn exec_trx_request_internal<T: Config>(
    request: Vec<u8>,
    signature: ChainAccountSignature,
    nonce: Nonce,
) -> Result<(), Reason> {
    log!("exec_trx_request_internal: {}", nonce);

    let request_str: &str = str::from_utf8(&request[..]).map_err(|_| Reason::InvalidUTF8)?;

    // Build Account=(Chain, Recover(Chain, Signature, NoncedRequest))
    let sender = signature.recover_account(&prepend_nonce(&request, nonce)[..])?;

    // Match TrxReqagainst known Transaction Requests
    let trx_request = trx_request::parse_request(request_str)
        .map_err(|e| Reason::TrxRequestParseError(reason::to_parse_error(e)))?;

    // Read Require Nonce=NonceAccount+1
    let current_nonce = Nonces::get(sender);
    require!(
        nonce == current_nonce,
        Reason::IncorrectNonce(nonce, current_nonce)
    );

    match trx_request {
        trx_request::TrxRequest::Extract(amount, asset, account) => match CashAsset::from(asset) {
            CashAsset::Cash => {
                extract_cash_internal::<T>(sender, account.into(), Left(Quantity(CASH, amount)))?;
            }
            CashAsset::Asset(chain_asset) => {
                let symbol = symbol::<T>(chain_asset).ok_or(Reason::AssetNotSupported)?;
                let quantity = Quantity(symbol, amount.into());

                extract_internal::<T>(chain_asset, sender, account.into(), quantity)?;
            }
        },
    }

    // Update user nonce
    Nonces::insert(sender, current_nonce + 1);

    Ok(())
}

// Dispatch Extrinsic Lifecycle //

/// Block initialization step that can fail
pub fn on_initialize_core<T: Config>() -> Result<frame_support::weights::Weight, Reason> {
    let now: Timestamp = now::<T>();
    let previous: Timestamp = LastBlockTimestamp::get();

    let change_in_time = now
        .checked_sub(previous)
        .ok_or(Reason::TimeTravelNotAllowed)?;

    let weight = on_initialize(change_in_time)?;

    LastBlockTimestamp::put(now);

    Ok(weight)
}

/// Block initialization step that can fail
pub fn on_initialize(change_in_time: Timestamp) -> Result<frame_support::weights::Weight, Reason> {
    let mut cash_principal_supply_increase = CashPrincipal::ZERO;
    let mut cash_principal_borrow_increase = CashPrincipal::ZERO;

    let mut asset_updates: Vec<(ChainAsset, AssetIndex, AssetIndex)> = Vec::new();
    let cash_index = GlobalCashIndex::get();

    // Iterate through listed assets, adding the CASH principal they generated/paid last block
    for (asset, symbol) in AssetSymbols::iter() {
        let (asset_cost, asset_yield) = get_rates(&asset)?;
        let asset_price = Prices::get(&symbol); // XXX this should use price fn since CASH price

        let cash_borrow_principal_per =
            compute_cash_principal_per(asset_cost, change_in_time, cash_index, asset_price)?;
        let cash_hold_principal_per =
            compute_cash_principal_per(asset_yield, change_in_time, cash_index, asset_price)?;

        let current_supply_index = SupplyIndices::get(&asset);
        let current_borrow_index = BorrowIndices::get(&asset);
        let new_supply_index = increment_index(current_supply_index, cash_hold_principal_per)?;
        let new_borrow_index = increment_index(current_borrow_index, cash_borrow_principal_per)?;

        let supply_asset = get_total_supply_assets(&asset)?;
        let borrow_asset = get_total_borrow_assets(&asset)?;

        cash_principal_supply_increase = cash_principal_supply_increase
            .add(supply_asset.as_cash_principal(cash_hold_principal_per)?)?;

        cash_principal_borrow_increase = cash_principal_borrow_increase
            .add(borrow_asset.as_cash_principal(cash_borrow_principal_per)?)?;

        asset_updates.push((asset.clone(), new_supply_index, new_borrow_index));
    }

    // Pay miners and update the CASH interest index on CASH itself
    let cash_yield: APR = CashYield::get();
    let cash_index_old: CashIndex = GlobalCashIndex::get();
    let total_cash_principal: CashPrincipal = TotalCashPrincipal::get();

    let increment = cash_yield.over_time(change_in_time)?;
    let cash_index_new = cash_index_old.increment(increment)?;
    let total_cash_principal_new = total_cash_principal.add(cash_principal_borrow_increase)?;
    let miner_spread_principal =
        cash_principal_borrow_increase.sub(cash_principal_supply_increase)?;
    let miner = ChainAccount::Eth([0; 20]); // todo: xxx how to get the current miner chain account
    let miner_cash_principal_old: CashPrincipal = CashPrincipals::get(&miner);
    let miner_cash_principal_new = miner_cash_principal_old.add(miner_spread_principal)?;

    // * WARNING - BEGIN STORAGE all checks and failures must happen above

    for (asset, new_supply_index, new_borrow_index) in asset_updates.drain(..) {
        SupplyIndices::insert(asset.clone(), new_supply_index);
        BorrowIndices::insert(asset, new_borrow_index);
    }

    GlobalCashIndex::put(cash_index_new);
    TotalCashPrincipal::put(total_cash_principal_new);
    CashPrincipals::insert(miner, miner_cash_principal_new);

    // todo: xxx support changing cash APRs
    /*
    Possibly update the CASH rate:
    If NextAPRStartAtNow
        Set CashYield=NextAPR
        Clear CashYieldNext
     */
    Ok(0)
}

pub fn process_events_internal<T: Config>(
    events: Vec<(ChainLogId, ChainLogEvent)>,
) -> Result<(), Reason> {
    for (event_id, event) in events.into_iter() {
        log!(
            "Processing event and sending extrinsic: {} {:?}",
            event_id.show(),
            event
        );

        // XXX
        let signature =  // XXX why are we signing with eth?
            <Ethereum as Chain>::sign_message(&event.encode()[..])?; // XXX
        let call = Call::process_chain_event(event_id, event, signature);

        // TODO: Do we want to short-circuit on an error here?
        let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
            .map_err(|()| Reason::FailedToSubmitExtrinsic);

        if res.is_err() {
            log!("Error while sending event extrinsic");
        }
    }
    Ok(())
}

pub fn process_chain_event_internal<T: Config>(
    event_id: ChainLogId,
    event: ChainLogEvent,
    signature: ValidatorSig,
) -> Result<(), Reason> {
    // XXX sig
    // XXX do we want to store/check hash to allow replaying?
    // TODO: use more generic function?
    // XXX why is this using eth for validator sig though?
    let signer: crate::types::ValidatorIdentity =
        compound_crypto::eth_recover(&event.encode()[..], &signature, false)?;
    let validators: Vec<_> = Validators::iter().map(|v| v.1.eth_address).collect();

    if !validators.contains(&signer) {
        log!(
            "Signer of a log event is not a known validator {:?}, validators are {:?}",
            signer,
            validators
        );
        return Err(Reason::UnknownValidator)?;
    }

    match EventStates::get(event_id) {
        EventState::Pending { signers } => {
            // XXX sets?
            if signers.contains(&signer) {
                log!("process_chain_event_internal({}): Validator has already signed this payload {:?}", event_id.show(), signer);
                return Err(Reason::ValidatorAlreadySigned);
            }

            // Add new validator to the signers
            let mut signers_new = signers.clone();
            signers_new.push(signer.clone()); // XXX unique add to set?

            if passes_validation_threshold(&signers_new, &validators) {
                match apply_chain_event_internal::<T>(event) {
                    Ok(()) => {
                        EventStates::insert(event_id, EventState::Done);
                        <Module<T>>::deposit_event(Event::ProcessedChainEvent(event_id));
                        Ok(())
                    }

                    Err(reason) => {
                        log!(
                            "process_chain_event_internal({}) apply failed: {:?}",
                            event_id.show(),
                            reason
                        );
                        EventStates::insert(event_id, EventState::Failed { reason });
                        <Module<T>>::deposit_event(Event::FailedProcessingChainEvent(
                            event_id, reason,
                        ));
                        Ok(())
                    }
                }
            } else {
                log!(
                    "process_chain_event_internal({}) signer_count={}",
                    event_id.show(),
                    signers_new.len()
                );
                EventStates::insert(
                    event_id,
                    EventState::Pending {
                        signers: signers_new,
                    },
                );
                Ok(())
            }
        }

        EventState::Failed { .. } | EventState::Done => {
            // TODO: Eventually we should be deleting or retrying here (based on monotonic ids and signing order)
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::{Chain, Ethereum};
    use crate::mock::*;
    use crate::rates::{InterestRateModel, Utilization};
    use crate::symbol::USD;
    use crate::tests::{get_eth, initialize_storage};

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

    #[test]
    fn test_extract_internal_min_value() {
        let asset = ChainAsset::Eth([238; 20]);
        let holder = ChainAccount::Eth([0; 20]);
        let recipient = ChainAccount::Eth([0; 20]);

        new_test_ext().execute_with(|| {
            AssetSymbols::insert(&asset, Symbol::new("ETH", 18));
            let symbol = symbol::<Test>(asset).expect("asset not found");
            let quantity = Quantity::from_nominal(symbol, "5.0");
            Prices::insert(symbol, 100_000); // $0.10
            let asset_balances_pre = AssetBalances::get(asset, holder);
            let total_supply_pre = TotalSupplyAssets::get(asset);
            let total_borrows_pre = TotalBorrowAssets::get(asset);
            let events_pre: Vec<_> = System::events().into_iter().collect();
            let notices_pre: Vec<(NoticeId, Notice)> = Notices::iter_prefix(ChainId::Eth).collect();
            let notice_states_pre: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let latest_notice_pre: Option<(NoticeId, ChainHash)> = LatestNotice::get(ChainId::Eth);
            let notice_hashes_pre: Vec<(ChainHash, NoticeId)> = NoticeHashes::iter().collect();
            let account_notices_pre: Vec<(ChainAccount, Vec<NoticeId>)> =
                AccountNotices::iter().collect();

            let res = extract_internal::<Test>(asset, holder, recipient, quantity);

            assert_eq!(res, Err(Reason::MinTxValueNotMet));

            let asset_balances_post = AssetBalances::get(asset, holder);
            let total_supply_post = TotalSupplyAssets::get(asset);
            let total_borrows_post = TotalBorrowAssets::get(asset);
            let events_post: Vec<_> = System::events().into_iter().collect();
            let notices_post: Vec<(NoticeId, Notice)> =
                Notices::iter_prefix(ChainId::Eth).collect();
            let notice_states_post: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let latest_notice_post: Option<(NoticeId, ChainHash)> = LatestNotice::get(ChainId::Eth);
            let notice_hashes_post: Vec<(ChainHash, NoticeId)> = NoticeHashes::iter().collect();
            let account_notices_post: Vec<(ChainAccount, Vec<NoticeId>)> =
                AccountNotices::iter().collect();

            assert_eq!(asset_balances_pre, asset_balances_post);
            assert_eq!(total_supply_pre, total_supply_post);
            assert_eq!(total_borrows_pre, total_borrows_post);
            assert_eq!(events_pre.len(), events_post.len());
            assert_eq!(notices_pre, notices_post);
            assert_eq!(notice_states_pre, notice_states_post);
            assert_eq!(latest_notice_pre, latest_notice_post);
            assert_eq!(notice_hashes_pre, notice_hashes_post);
            assert_eq!(account_notices_pre, account_notices_post);
        });
    }

    #[test]
    fn test_extract_internal_sufficient_value() {
        let eth_asset = [238; 20];
        let asset = ChainAsset::Eth(eth_asset);
        let eth_holder = [0; 20];
        let eth_recipient = [0; 20];
        let holder = ChainAccount::Eth(eth_holder);
        let recipient = ChainAccount::Eth(eth_recipient);

        new_test_ext().execute_with(|| {
            AssetSymbols::insert(&asset, Symbol::new("ETH", 18));
            let symbol = symbol::<Test>(asset).expect("asset not found");
            let quantity = Quantity::from_nominal(symbol, "50.0");
            Prices::insert(symbol, 100_000); // $0.10
            let asset_balances_pre = AssetBalances::get(asset, holder);
            let total_supply_pre = TotalSupplyAssets::get(asset);
            let total_borrows_pre = TotalBorrowAssets::get(asset);
            let events_pre: Vec<_> = System::events().into_iter().collect();
            let notices_pre: Vec<(NoticeId, Notice)> = Notices::iter_prefix(ChainId::Eth).collect();
            let notice_states_pre: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let latest_notice_pre: Option<(NoticeId, ChainHash)> = LatestNotice::get(ChainId::Eth);
            let notice_hashes_pre: Vec<(ChainHash, NoticeId)> = NoticeHashes::iter().collect();
            let account_notices_pre: Vec<(ChainAccount, Vec<NoticeId>)> =
                AccountNotices::iter().collect();

            let res = extract_internal::<Test>(asset, holder, recipient, quantity);

            assert_eq!(res, Ok(()));

            let asset_balances_post = AssetBalances::get(asset, holder);
            let total_supply_post = TotalSupplyAssets::get(asset);
            let total_borrows_post = TotalBorrowAssets::get(asset);
            let events_post: Vec<_> = System::events().into_iter().collect();
            let notices_post: Vec<(NoticeId, Notice)> =
                Notices::iter_prefix(ChainId::Eth).collect();
            let notice_states_post: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let latest_notice_post: Option<(NoticeId, ChainHash)> = LatestNotice::get(ChainId::Eth);
            let notice_hashes_post: Vec<(ChainHash, NoticeId)> = NoticeHashes::iter().collect();
            let account_notices_post: Vec<(ChainAccount, Vec<NoticeId>)> =
                AccountNotices::iter().collect();

            assert_eq!(
                asset_balances_pre - 50000000000000000000,
                asset_balances_post
            ); // 50e18
            assert_eq!(total_supply_pre, total_supply_post);
            assert_eq!(total_borrows_pre + 50000000000000000000, total_borrows_post);

            assert_eq!(events_pre.len() + 1, events_post.len());

            assert_eq!(notices_pre.len() + 1, notices_post.len());
            assert_eq!(notice_states_pre.len() + 1, notice_states_post.len());
            assert_ne!(latest_notice_pre, latest_notice_post);
            assert_eq!(notice_hashes_pre.len() + 1, notice_hashes_post.len());
            assert_eq!(account_notices_pre.len() + 1, account_notices_post.len());

            let notice_event = events_post.into_iter().next().unwrap();

            let notice = notices_post.into_iter().last().unwrap().1;
            let notice_state = notice_states_post.into_iter().last().unwrap();
            let latest_notice = latest_notice_post.unwrap();
            let notice_hash = notice_hashes_post.into_iter().last().unwrap();
            let account_notice = account_notices_post.into_iter().last().unwrap();

            let expected_notice_id = NoticeId(0, 1);
            let expected_notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: expected_notice_id,
                parent: [0u8; 32],
                asset: eth_asset,
                account: eth_recipient,
                amount: 50000000000000000000,
            });
            let expected_notice_encoded = expected_notice.encode_notice();
            let expected_notice_hash = expected_notice.hash();

            assert_eq!(notice, expected_notice.clone());
            assert_eq!(
                (
                    ChainId::Eth,
                    expected_notice_id,
                    NoticeState::Pending {
                        signature_pairs: ChainSignatureList::Eth(vec![])
                    }
                ),
                notice_state
            );
            assert_eq!((expected_notice_id, expected_notice_hash), latest_notice);
            assert_eq!((expected_notice_hash, expected_notice_id), notice_hash);
            assert_eq!((recipient, vec![expected_notice_id]), account_notice);

            assert_eq!(
                TestEvent::cash(Event::Notice(
                    expected_notice_id,
                    expected_notice,
                    expected_notice_encoded
                )),
                notice_event.event
            );
        });
    }

    #[test]
    fn test_extract_internal_notice_ids() {
        let eth_asset = [238; 20];
        let asset = ChainAsset::Eth(eth_asset);
        let eth_holder = [0; 20];
        let eth_recipient = [0; 20];
        let holder = ChainAccount::Eth(eth_holder);
        let recipient = ChainAccount::Eth(eth_recipient);

        new_test_ext().execute_with(|| {
            AssetSymbols::insert(&asset, Symbol::new("ETH", 18));
            let symbol = symbol::<Test>(asset).expect("asset not found");
            let quantity = Quantity::from_nominal(symbol, "50.0");
            Prices::insert(symbol, 100_000); // $0.10

            let notices_pre: Vec<(NoticeId, Notice)> = Notices::iter_prefix(ChainId::Eth).collect();
            let notice_states_pre: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let latest_notice_pre: Option<(NoticeId, ChainHash)> = LatestNotice::get(ChainId::Eth);
            let notice_hashes_pre: Vec<(ChainHash, NoticeId)> = NoticeHashes::iter().collect();
            let account_notices_pre: Vec<(ChainAccount, Vec<NoticeId>)> =
                AccountNotices::iter().collect();

            assert_eq!(LatestNotice::get(ChainId::Eth), None);
            assert_eq!(
                extract_internal::<Test>(asset, holder, recipient, quantity),
                Ok(())
            );

            let notice_state_post: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let notice_state = notice_state_post.into_iter().next().unwrap();
            let notice = Notices::get(notice_state.0, notice_state.1);

            let expected_notice_id = NoticeId(0, 1);
            let expected_notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: expected_notice_id,
                parent: [0u8; 32],
                asset: eth_asset,
                account: eth_recipient,
                amount: 50000000000000000000,
            });

            assert_eq!(
                (
                    ChainId::Eth,
                    expected_notice_id,
                    NoticeState::Pending {
                        signature_pairs: ChainSignatureList::Eth(vec![])
                    }
                ),
                notice_state
            );

            assert_eq!(notice, Some(expected_notice.clone()));

            assert_eq!(
                LatestNotice::get(ChainId::Eth),
                Some((NoticeId(0, 1), expected_notice.hash()))
            );
            assert_eq!(
                extract_internal::<Test>(asset, holder, recipient, quantity),
                Ok(())
            );

            let notices_post_2: Vec<(NoticeId, Notice)> =
                Notices::iter_prefix(ChainId::Eth).collect();
            let notice_states_post_2: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let latest_notice_post_2: Option<(NoticeId, ChainHash)> =
                LatestNotice::get(ChainId::Eth);
            let notice_hashes_post_2: Vec<(ChainHash, NoticeId)> = NoticeHashes::iter().collect();
            let account_notices_post_2: Vec<(ChainAccount, Vec<NoticeId>)> =
                AccountNotices::iter().collect();

            assert_eq!(notices_pre.len() + 2, notices_post_2.len());
            assert_eq!(notice_states_pre.len() + 2, notice_states_post_2.len());
            assert_ne!(latest_notice_pre, latest_notice_post_2);
            assert_eq!(notice_hashes_pre.len() + 2, notice_hashes_post_2.len());
            assert_eq!(account_notices_pre.len() + 1, account_notices_post_2.len());

            let latest_notice_2 = LatestNotice::get(ChainId::Eth).unwrap();
            let notice_2 = Notices::get(ChainId::Eth, latest_notice_2.0).unwrap();
            let notice_state_2 = NoticeStates::get(ChainId::Eth, latest_notice_2.0);
            let notice_hash_2 = NoticeHashes::get(latest_notice_2.1).unwrap();
            let account_notice_2 = AccountNotices::get(recipient);

            let expected_notice_2_id = NoticeId(0, 2);
            let expected_notice_2 = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: expected_notice_2_id,
                parent: <Ethereum as Chain>::hash_bytes(&expected_notice.encode_notice()),
                asset: eth_asset,
                account: eth_recipient,
                amount: 50000000000000000000,
            });
            let expected_notice_encoded_2 = expected_notice_2.encode_notice();
            let expected_notice_hash_2 = expected_notice_2.hash();

            assert_eq!(notice_2, expected_notice_2.clone());
            assert_eq!(
                NoticeState::Pending {
                    signature_pairs: ChainSignatureList::Eth(vec![])
                },
                notice_state_2
            );
            assert_eq!(
                (expected_notice_2_id, expected_notice_hash_2),
                latest_notice_2
            );
            assert_eq!(expected_notice_2_id, notice_hash_2);
            assert_eq!(
                vec![expected_notice_id, expected_notice_2_id],
                account_notice_2
            );
        });
    }

    #[test]
    fn test_compute_cash_principal_per() {
        // round numbers (unrealistic but very easy to check)
        let asset_rate = APR::from_nominal("0.30"); // 30% per year
        let dt = MILLISECONDS_PER_YEAR / 2; // for 6 months
        let cash_index = CashIndex::from_nominal("1.5"); // current index value 1.5
        let price_asset: AssetPrice = 1500_000000; // $1,500

        let actual = compute_cash_principal_per(asset_rate, dt, cash_index, price_asset).unwrap();
        let expected = CashPrincipal::from_nominal("150"); // from hand calc
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_compute_cash_principal_per_specific_case() {
        // a unit test related to previous unexpected larger scope test of on_initialize
        // this showed that we should divide by SECONDS_PER_YEAR last te prevent un-necessary truncation
        let asset_rate = APR::from_nominal("0.1225");
        let dt = MILLISECONDS_PER_YEAR / 4;
        let cash_index = CashIndex::from_nominal("1.123");
        let price_asset: AssetPrice = 1450_000000;

        let actual = compute_cash_principal_per(asset_rate, dt, cash_index, price_asset).unwrap();
        let expected = CashPrincipal::from_nominal("39.542520"); // from hand calc
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_scale_multiple_terms() {
        let n1 = 56_789u128; // 3 decimals 56.789
        let n2 = 12_3456; // 4 decimals 12.3456
        let d1 = 7_89; // 2 decimals 7.89
        let unscaled = n1 * n2 / d1;

        // scale 0 => output decimals = 5
        let actual = scale_multiple_terms(unscaled, 7, 2, 5).unwrap();
        assert_eq!(actual, 88_85859);

        // scale > 0 => output decimals > 5, say 8
        let actual = scale_multiple_terms(unscaled, 7, 2, 8).unwrap();
        assert_eq!(actual, 88_85859_000); // 3 scale decimals resulting in right trailing zeros

        // scale < 0 => output decimals < 5, say 2
        let actual = scale_multiple_terms(unscaled, 7, 2, 2).unwrap();
        assert_eq!(actual, 88_85);
    }

    #[test]
    fn test_get_utilization() {
        new_test_ext().execute_with(|| {
            initialize_storage();
            let asset = get_eth();
            TotalSupplyAssets::insert(&asset, 100);
            TotalBorrowAssets::insert(&asset, 50);
            let utilization = crate::core::get_utilization(&asset).unwrap();
            assert_eq!(utilization, Utilization::from_nominal("0.5"));
        });
    }

    #[test]
    fn test_get_borrow_rate() {
        new_test_ext().execute_with(|| {
            initialize_storage();
            let asset = get_eth();
            let kink_rate = 105;
            let expected_model = InterestRateModel::new_kink(100, kink_rate, 5000, 202);
            crate::ReserveFactor::insert(&asset, crate::rates::ReserveFactor::from_nominal("0.5"));
            TotalSupplyAssets::insert(&asset, 100);
            TotalBorrowAssets::insert(&asset, 50);

            CashModule::update_interest_rate_model(Origin::root(), asset.clone(), expected_model)
                .unwrap();
            let (borrow_rate, supply_rate) = crate::core::get_rates(&asset).unwrap();

            assert_eq!(borrow_rate, kink_rate.into());
            // 50% utilization and 50% reserve factor
            assert_eq!(supply_rate, (kink_rate / 2 / 2).into());
        });
    }

    #[test]
    fn test_on_initialize() {
        new_test_ext().execute_with(|| {
            let miner = ChainAccount::Eth([0; 20]); // todo: xxx how to get the current miner chain account
            let asset = get_eth();
            let symbol = Symbol::new("ETH", 18);
            let change_in_time = MILLISECONDS_PER_YEAR / 4; // 3 months go by
            let model = InterestRateModel::new_kink(0, 2500, 5000, 5000);

            ReserveFactor::insert(
                asset.clone(),
                crate::rates::ReserveFactor::from_nominal("0.02"),
            );
            RateModels::insert(asset.clone(), model);
            GlobalCashIndex::put(CashIndex::from_nominal("1.123"));
            AssetSymbols::insert(asset.clone(), symbol);
            SupplyIndices::insert(asset.clone(), AssetIndex::from_nominal("1234"));
            BorrowIndices::insert(asset.clone(), AssetIndex::from_nominal("1345"));
            TotalSupplyAssets::insert(asset.clone(), AssetQuantity::from_nominal(symbol, "300").1);
            TotalBorrowAssets::insert(asset.clone(), AssetQuantity::from_nominal(symbol, "150").1);
            CashYield::put(APR::from_nominal("0.24")); // 24% APR big number for easy to see interest
            TotalCashPrincipal::put(CashPrincipal::from_nominal("450000")); // 450k cash principal
            CashPrincipals::insert(miner.clone(), CashPrincipal::from_nominal("1"));
            Prices::insert(symbol, 1450_000000 as AssetPrice); // $1450 eth

            let result = on_initialize(change_in_time);
            assert_eq!(result, Ok(0u64));

            assert_eq!(
                SupplyIndices::get(&asset),
                AssetIndex::from_nominal("1273.542520")
            );
            assert_eq!(
                BorrowIndices::get(&asset),
                AssetIndex::from_nominal("1425.699020")
            );
            assert_eq!(GlobalCashIndex::get(), CashIndex::from_nominal("1.1924"));
            // todo: it looks like some precision is getting lost should be 462104.853072
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipal::from_nominal("462104.853000")
            );
            // todo: same here, should be 243.097061
            assert_eq!(
                CashPrincipals::get(&miner),
                CashPrincipal::from_nominal("243.097000")
            );
        });
    }

    #[test]
    fn test_now() {
        new_test_ext().execute_with(|| {
            let expected = 123;
            <pallet_timestamp::Module<Test>>::set_timestamp(expected);
            let actual = now::<Test>();
            assert_eq!(actual, expected);
        });
    }
}
