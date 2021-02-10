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
        eth, ChainAccount, ChainAccountSignature, ChainAsset, ChainAssetAccount, ChainHash,
        ChainId, ChainSignature, ChainSignatureList,
    },
    log, notices,
    notices::{EncodeNotice, ExtractionNotice, Notice, NoticeId, NoticeState},
    params::MIN_TX_VALUE,
    reason,
    reason::{MathError, Reason},
    symbol::{Symbol, CASH},
    types::{
        AssetAmount, AssetBalance, AssetQuantity, CashPrincipal, CashQuantity, EthAddress, Int,
        Nonce, Price, Quantity, SafeMul, USDQuantity,
    },
    AccountNotices, AssetBalances, AssetSymbols, BorrowIndices, Call, CashPrincipals, CashYield,
    ChainCashPrincipals, Config, Event, GlobalCashIndex, LastBlockTimestamp, LatestNotice, Module,
    Nonces, NoticeStates, Notices, Prices, RateModels, ReserveFactor, SubmitTransaction,
    SupplyIndices, TotalBorrowAssets, TotalCashPrincipal, TotalSupplyAssets,
};
use frame_support::sp_runtime::traits::Convert;
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
    signers: &Vec<EthAddress>,
    validators: &Vec<EthAddress>,
) -> bool {
    let mut signer_set = BTreeSet::<EthAddress>::new();
    for v in signers {
        signer_set.insert(*v);
    }

    let mut validator_set = BTreeSet::<EthAddress>::new();
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
            Quantity(symbol::<T>(ChainAsset::Eth(asset)).ok_or(Reason::NoSuchAsset)?, amount),
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
    <Module<T>>::deposit_event(Event::GoldieLocks(asset, holder, amount.value())); // XXX -> raw amount?
    Ok(())
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

    AssetBalances::insert(asset, holder, holder_asset_new);
    TotalSupplyAssets::insert(asset, total_supply_new);
    TotalBorrowAssets::insert(asset, total_borrow_new);

    // TODO: Consider how to better factor this
    let chain_id = recipient.chain_id();
    let (latest_notice_id, parent_hash) =
        LatestNotice::get(chain_id).unwrap_or((NoticeId(0, 0), chain_id.zero_hash()));
    let notice_id = latest_notice_id.seq();

    let notice = Notice::ExtractionNotice(match (chain_asset_account, parent_hash) {
        (ChainAssetAccount::Eth(eth_asset, eth_account), ChainHash::Eth(eth_parent_hash)) => {
            Ok(ExtractionNotice::Eth {
                id: notice_id,
                parent: eth_parent_hash,
                asset: eth_asset,
                account: eth_account,
                amount: amount.1,
            })
        }
        _ => Err(Reason::AssetExtractionNotSupported),
    }?);

    // Add to notices, notice states, track the latest notice and index by account
    Notices::insert(chain_id, notice_id, &notice);
    NoticeStates::insert(chain_id, notice_id, NoticeState::pending(&notice));
    LatestNotice::insert(chain_id, (notice_id, notice.hash()));
    AccountNotices::append(recipient, notice_id);

    // Deposit Notice Event
    let encoded_notice = notice.encode_notice();
    Module::<T>::deposit_event(Event::Notice(notice_id, notice, encoded_notice));

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

    for (asset, symbol) in AssetSymbols::iter() {
        // note we do not need to check that the asset is supported because it is by definition
        // of where we got it from
        let (asset_cost, asset_yield) = get_rates(&asset)?;
        let asset_price = Prices::get(&symbol);

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

        // CashPrincipal__Increase = CashPrincipal__Increase + ___Asset * Cash__PrincipalPer

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

// Liquidity Checks //

pub fn has_liquidity_to_reduce_asset(_holder: ChainAccount, _amount: AssetQuantity) -> bool {
    true // XXX
}

pub fn has_liquidity_to_reduce_cash(_holder: ChainAccount, _amount: CashQuantity) -> bool {
    true // XXX
}

// Off chain worker //

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
        trx_request::TrxRequest::Extract(amount, asset, account) => {
            let chain_asset: ChainAsset = asset.into();
            let symbol = symbol::<T>(chain_asset).ok_or(Reason::AssetNotSupported)?;
            let quantity = Quantity(symbol, amount.into());

            extract_internal::<T>(chain_asset, sender, account.into(), quantity)?;
        }
    }

    // Update user nonce
    Nonces::insert(sender, current_nonce + 1);

    Ok(())
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
            let notice_states_pre: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let events_pre: Vec<_> = System::events().into_iter().collect();

            let res = extract_internal::<Test>(asset, holder, recipient, quantity);

            assert_eq!(res, Err(Reason::MinTxValueNotMet));

            let asset_balances_post = AssetBalances::get(asset, holder);
            let total_supply_post = TotalSupplyAssets::get(asset);
            let total_borrows_post = TotalBorrowAssets::get(asset);
            let notice_states_post: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let events_post: Vec<_> = System::events().into_iter().collect();

            assert_eq!(asset_balances_pre, asset_balances_post);
            assert_eq!(total_supply_pre, total_supply_post);
            assert_eq!(total_borrows_pre, total_borrows_post);
            assert_eq!(notice_states_pre.len(), notice_states_post.len());
            assert_eq!(events_pre.len(), events_post.len());
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
            let notice_states_pre: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let events_pre: Vec<_> = System::events().into_iter().collect();

            let res = extract_internal::<Test>(asset, holder, recipient, quantity);

            assert_eq!(res, Ok(()));

            let asset_balances_post = AssetBalances::get(asset, holder);
            let total_supply_post = TotalSupplyAssets::get(asset);
            let total_borrows_post = TotalBorrowAssets::get(asset);
            let notice_states_post: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let events_post: Vec<_> = System::events().into_iter().collect();

            assert_eq!(
                asset_balances_pre - 50000000000000000000,
                asset_balances_post
            ); // 50e18
            assert_eq!(total_supply_pre, total_supply_post);
            assert_eq!(total_borrows_pre + 50000000000000000000, total_borrows_post);
            assert_eq!(notice_states_pre.len() + 1, notice_states_post.len());
            assert_eq!(events_pre.len() + 1, events_post.len());

            let notice_state = notice_states_post.into_iter().next().unwrap();
            let notice = Notices::get(notice_state.0, notice_state.1);
            let notice_event = events_post.into_iter().next().unwrap();

            let expected_notice_id = NoticeId(0, 1);
            let expected_notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: expected_notice_id,
                parent: [0u8; 32],
                asset: eth_asset,
                account: eth_recipient,
                amount: 50000000000000000000,
            });
            let expected_notice_encoded = expected_notice.encode_notice();

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

            let notice_state_post_2: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let notice_state_2 = notice_state_post_2.into_iter().next().unwrap();
            let notice_2 = Notices::get(notice_state_2.0, notice_state_2.1);

            let expected_notice_id_2 = NoticeId(0, 2);
            let expected_notice_2 = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: expected_notice_id_2,
                parent: <Ethereum as Chain>::hash_bytes(&expected_notice.encode_notice()),
                asset: eth_asset,
                account: eth_recipient,
                amount: 50000000000000000000,
            });

            assert_eq!(
                (
                    ChainId::Eth,
                    expected_notice_id_2,
                    NoticeState::Pending {
                        signature_pairs: ChainSignatureList::Eth(vec![])
                    }
                ),
                notice_state_2
            );

            assert_eq!(notice_2, Some(expected_notice_2.clone()));

            assert_eq!(
                LatestNotice::get(ChainId::Eth),
                Some((NoticeId(0, 2), expected_notice_2.hash()))
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
