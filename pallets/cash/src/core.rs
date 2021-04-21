// Note: The substrate build requires these be re-exported.
pub use our_std::{
    cmp::{max, min},
    collections::btree_set::BTreeSet,
    convert::{TryFrom, TryInto},
    fmt, if_std, result,
    result::Result,
    str,
};

#[cfg(feature = "freeze-time")]
use std::{env, fs};

use codec::Decode;
use frame_support::traits::UnfilteredDispatchable;
use frame_support::{
    sp_runtime::traits::Convert,
    storage::{
        IterableStorageDoubleMap, IterableStorageMap, StorageDoubleMap, StorageMap, StorageValue,
    },
};

use pallet_oracle::types::Price;

use crate::{
    chains::{
        Chain, ChainAccount, ChainAsset, ChainBlockEvent, ChainBlockEvents, ChainHash, ChainId,
        ChainSignature, Ethereum,
    },
    factor::Factor,
    internal::{self, balance_helpers::*, liquidity},
    log,
    params::{MIN_TX_VALUE, TRANSFER_FEE},
    pipeline::CashPipeline,
    portfolio::Portfolio,
    rates::APR,
    reason::{MathError, Reason},
    types::{
        AssetAmount, AssetBalance, AssetIndex, AssetInfo, AssetQuantity, Balance, CashIndex,
        CashPrincipal, CashPrincipalAmount, GovernanceResult, NoticeId, Quantity, SignersSet,
        Timestamp, USDQuantity, Units, ValidatorKeys, CASH,
    },
    AssetBalances, AssetsWithNonZeroBalance, BorrowIndices, CashPrincipals, CashYield,
    CashYieldNext, ChainCashPrincipals, Config, Event, GlobalCashIndex, IngressionQueue,
    LastBlockTimestamp, LastIndices, LastMinerSharePrincipal, LastYieldCashIndex,
    LastYieldTimestamp, Miner, Module, SupplyIndices, SupportedAssets, TotalBorrowAssets,
    TotalCashPrincipal, TotalSupplyAssets, Validators,
};

#[macro_export]
macro_rules! require {
    ($expr:expr, $reason:expr) => {
        if !$expr {
            return core::result::Result::Err($reason);
        }
    };
}

#[macro_export]
macro_rules! require_min_tx_value {
    ($value:expr) => {
        require!($value >= MIN_TX_VALUE, Reason::MinTxValueNotMet);
    };
}

// Public helper functions //

// Miner might not be set (e.g. in the first block mined), but for accouting
// purposes, we want some address to make sure all numbers tie out. As such,
// let's just give the initial rewards to some burn account.
pub fn get_some_miner<T: Config>() -> ChainAccount {
    Miner::get().unwrap_or(ChainAccount::Eth([0; 20]))
}

#[cfg(feature = "freeze-time")]
pub fn get_now<T: Config>() -> Timestamp {
    if_std! {
        if let Ok(filename) = env::var("FREEZE_TIME") {
            if let Ok(contents) = fs::read_to_string(filename) {
                if let Ok(time) = contents.parse::<u64>() {
                    println!("Freeze Time: {}", time);
                    if time > 0 {
                        return time;
                    }
                }
            }
        }
    }

    let now = <pallet_timestamp::Module<T>>::get();
    T::TimeConverter::convert(now)
}

#[cfg(not(feature = "freeze-time"))]
pub fn get_now<T: Config>() -> Timestamp {
    let now = <pallet_timestamp::Module<T>>::get();
    T::TimeConverter::convert(now)
}

/// Return the full asset info for an asset.
pub fn get_asset<T: Config>(asset: ChainAsset) -> Result<AssetInfo, Reason> {
    Ok(SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?)
}

/// Return the USD price associated with the given units.
pub fn get_price<T: pallet_oracle::Config>(units: Units) -> Result<Price, Reason> {
    pallet_oracle::get_price_by_ticker::<T>(units.ticker).ok_or(Reason::NoPrice)
}

/// Return the price or zero if not given
pub fn get_price_or_zero<T: pallet_oracle::Config>(units: Units) -> Price {
    pallet_oracle::get_price_by_ticker::<T>(units.ticker).unwrap_or(Price::new(units.ticker, 0))
}

/// Return a quantity with units of the given asset.
pub fn get_quantity<T: Config>(asset: ChainAsset, amount: AssetAmount) -> Result<Quantity, Reason> {
    Ok(SupportedAssets::get(asset)
        .ok_or(Reason::AssetNotSupported)?
        .as_quantity(amount))
}

/// Return a quantity with units of CASH.
pub fn get_cash_quantity<T: Config>(principal: CashPrincipalAmount) -> Result<Quantity, Reason> {
    Ok(GlobalCashIndex::get().cash_quantity(principal)?)
}

/// Return the USD value of the asset amount.
pub fn get_value<T: Config>(amount: AssetQuantity) -> Result<USDQuantity, Reason> {
    Ok(amount.mul_price(get_price::<T>(amount.units)?)?)
}

/// Return the current utilization for the asset.
pub fn get_utilization<T: Config>(asset: ChainAsset) -> Result<Factor, Reason> {
    let _info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    let total_supply = TotalSupplyAssets::get(asset);
    let total_borrow = TotalBorrowAssets::get(asset);
    Ok(crate::rates::get_utilization(total_supply, total_borrow)?)
}

/// Return the current borrow and supply rates for the asset.
pub fn get_rates<T: Config>(asset: ChainAsset) -> Result<(APR, APR), Reason> {
    let info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    let utilization = get_utilization::<T>(asset)?;
    Ok(info
        .rate_model
        .get_rates(utilization, APR::ZERO, info.miner_shares)?)
}

/// Return the current list of assets.
pub fn get_assets<T: Config>() -> Result<Vec<AssetInfo>, Reason> {
    let info = SupportedAssets::iter()
        .map(|(_chain_asset, asset_info)| asset_info)
        .collect::<Vec<AssetInfo>>();
    Ok(info)
}

/// Return the event ingression queue for the underlying chain.
pub fn get_event_queue<T: Config>(chain_id: ChainId) -> Result<ChainBlockEvents, Reason> {
    Ok(IngressionQueue::get(chain_id).unwrap_or(ChainBlockEvents::empty(chain_id)))
}

/// Return the current total borrow and total supply balances for the asset.
pub fn get_market_totals<T: Config>(
    asset: ChainAsset,
) -> Result<(AssetAmount, AssetAmount), Reason> {
    let _info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    let total_borrow = TotalBorrowAssets::get(asset);
    let total_supply = TotalSupplyAssets::get(asset);
    Ok((total_borrow, total_supply))
}

/// Return the account's balance for the asset.
pub fn get_account_balance<T: Config>(
    account: ChainAccount,
    asset: ChainAsset,
) -> Result<AssetBalance, Reason> {
    let _info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    Ok(AssetBalances::get(asset, account))
}

/// Return the current cash yield.
pub fn get_cash_yield<T: Config>() -> Result<APR, Reason> {
    Ok(CashYield::get())
}

/// Return the current borrow and supply rates for the asset.
pub fn get_accounts<T: Config>() -> Result<Vec<ChainAccount>, Reason> {
    let info: Vec<ChainAccount> = CashPrincipals::iter()
        .map(|p| p.0)
        .collect::<Vec<ChainAccount>>();
    Ok(info)
}

/// Return the current borrow and supply rates for the asset.
pub fn get_accounts_liquidity<T: Config>() -> Result<Vec<(ChainAccount, AssetBalance)>, Reason> {
    let mut info: Vec<(ChainAccount, AssetBalance)> = CashPrincipals::iter()
        .map(|a| {
            (
                a.0.clone(),
                liquidity::get_liquidity::<T>(a.0).unwrap().value,
            )
        })
        .collect::<Vec<(ChainAccount, AssetBalance)>>();
    info.sort_by(|(_a_account, a_balance), (_b_account, b_balance)| a_balance.cmp(b_balance));
    Ok(info)
}

/// Return the portfolio of the chain account
pub fn get_portfolio<T: Config>(account: ChainAccount) -> Result<Portfolio, Reason> {
    Ok(Portfolio::from_storage::<T>(account)?)
}

pub fn get_validator_set<T: Config>() -> Result<SignersSet, Reason> {
    // Note: inefficient, probably manage reading validators from storage better
    Ok(Validators::iter().map(|(_, v)| v.substrate_id).collect())
}

/// Return the validator which signed the given data, given signature.
pub fn recover_validator<T: Config>(
    data: &[u8],
    signature: ChainSignature,
) -> Result<ValidatorKeys, Reason> {
    // Note: inefficient, we should index by every key we want to query by
    match signature {
        ChainSignature::Eth(eth_sig) => {
            let signer = <Ethereum as Chain>::recover_address(data, eth_sig)?;
            for (_, validator) in Validators::iter() {
                if validator.eth_address == signer {
                    return Ok(validator);
                }
            }
        }

        _ => {
            // XXX should we even be having other branches for signatures now?
            //  we should probably delete all variants of ChainSignature except Eth
            //   generally minimal since we dont want validators to have to add new keys
            //    can be separate from ChainAccountSignature
            return Err(Reason::NotImplemented); // XXX
        }
    }

    Err(Reason::UnknownValidator)
}

/// Sign the given data as a validator, assuming we have the credentials.
/// The validator can sign with any valid ChainSignature, which happens to only be Eth currently.
pub fn validator_sign<T: Config>(data: &[u8]) -> Result<ChainSignature, Reason> {
    Ok(ChainSignature::Eth(<Ethereum as Chain>::sign_message(
        data,
    )?))
}

pub fn get_chain_account(chain: String, recipient: [u8; 32]) -> Result<ChainAccount, Reason> {
    match &chain.to_ascii_uppercase()[..] {
        "ETH" => {
            let mut eth_recipient: [u8; 20] = [0; 20];
            eth_recipient[..].clone_from_slice(&recipient[0..20]);

            Ok(ChainAccount::Eth(eth_recipient))
        }
        _ => Err(Reason::InvalidChain),
    }
}

// Protocol interface //

/// Apply the event to the current state, effectively taking the action.
pub fn apply_chain_event_internal<T: Config>(event: &ChainBlockEvent) -> Result<(), Reason> {
    log!("apply_chain_event_internal(event): {:?}", event);

    match event {
        ChainBlockEvent::Eth(_block_num, eth_event) => match eth_event {
            ethereum_client::EthereumEvent::Lock {
                asset,
                sender,
                chain,
                recipient,
                amount,
            } => internal::lock::lock_internal::<T>(
                get_asset::<T>(ChainAsset::Eth(*asset))?,
                ChainAccount::Eth(*sender),
                get_chain_account(chain.to_string(), *recipient)?,
                get_quantity::<T>(ChainAsset::Eth(*asset), *amount)?,
            ),

            ethereum_client::EthereumEvent::LockCash {
                sender,
                chain,
                recipient,
                principal,
                ..
            } => internal::lock::lock_cash_principal_internal::<T>(
                ChainAccount::Eth(*sender),
                get_chain_account(chain.to_string(), *recipient)?,
                CashPrincipalAmount(*principal),
            ),

            ethereum_client::EthereumEvent::ExecuteProposal {
                title: _title,
                extrinsics,
            } => dispatch_extrinsics_internal::<T>(extrinsics.to_vec()),

            ethereum_client::EthereumEvent::ExecTrxRequest {
                account,
                trx_request,
            } => internal::exec_trx_request::exec_trx_request::<T>(
                &trx_request[..],
                ChainAccount::Eth(*account),
                None,
            ),

            ethereum_client::EthereumEvent::NoticeInvoked {
                era_id,
                era_index,
                notice_hash,
                result,
            } => internal::notices::handle_notice_invoked::<T>(
                ChainId::Eth,
                NoticeId(*era_id, *era_index),
                ChainHash::Eth(*notice_hash),
                result.to_vec(),
            ),
        },
    }
}

/// Un-apply the event on the current state, undoing the action to the extent possible/necessary.
pub fn unapply_chain_event_internal<T: Config>(event: &ChainBlockEvent) -> Result<(), Reason> {
    log!("unapply_chain_event_internal(event): {:?}", event);

    match event {
        ChainBlockEvent::Eth(_block_num, eth_event) => match eth_event {
            ethereum_client::EthereumEvent::Lock {
                asset,
                sender,
                chain,
                recipient,
                amount,
            } => internal::lock::undo_lock_internal::<T>(
                get_asset::<T>(ChainAsset::Eth(*asset))?,
                ChainAccount::Eth(*sender),
                get_chain_account(chain.to_string(), *recipient)?,
                get_quantity::<T>(ChainAsset::Eth(*asset), *amount)?,
            ),

            ethereum_client::EthereumEvent::LockCash {
                sender,
                chain,
                recipient,
                principal,
                ..
            } => internal::lock::undo_lock_cash_principal_internal::<T>(
                ChainAccount::Eth(*sender),
                get_chain_account(chain.to_string(), *recipient)?,
                CashPrincipalAmount(*principal),
            ),

            _ => Ok(()),
        },
    }
}

/// Update the index of which assets an account has non-zero balances in.
pub fn set_asset_balance_internal<T: Config>(
    asset: ChainAsset,
    account: ChainAccount,
    balance: AssetBalance,
) {
    if balance == 0 {
        AssetsWithNonZeroBalance::remove(account, asset);
    } else {
        AssetsWithNonZeroBalance::insert(account, asset, ());
    }

    AssetBalances::insert(asset, account, balance);
}

pub fn dispatch_extrinsics_internal<T: Config>(extrinsics: Vec<Vec<u8>>) -> Result<(), Reason> {
    // Decode a SCALE-encoded set of extrinsics from the event
    // For each extrinsic, dispatch the given extrinsic as Root
    let results: Vec<(Vec<u8>, GovernanceResult)> = extrinsics
        .into_iter()
        .map(|payload| {
            log!(
                "dispatch_extrinsics_internal:: dispatching extrinsic {}",
                hex::encode(&payload)
            );
            let call_res: Result<<T as Config>::Call, _> = Decode::decode(&mut &payload[..]);
            match call_res {
                Ok(call) => {
                    log!("dispatch_extrinsics_internal:: dispatching {:?}", call);
                    let res = call.dispatch_bypass_filter(frame_system::RawOrigin::Root.into());

                    let gov_res = match res {
                        Ok(_) => GovernanceResult::DispatchSuccess,
                        Err(error_with_post_info) => {
                            GovernanceResult::DispatchFailure(error_with_post_info.error)
                        }
                    };

                    log!("dispatch_extrinsics_internal:: res {:?}", res);
                    (payload, gov_res)
                }
                _ => {
                    log!(
                        "dispatch_extrinsics_internal:: failed to decode extrinsic {}",
                        hex::encode(&payload)
                    );
                    (payload, GovernanceResult::FailedToDecodeCall)
                }
            }
        })
        .collect();

    <Module<T>>::deposit_event(Event::ExecutedGovernance(results));

    Ok(())
}

pub fn extract_internal<T: Config>(
    asset: AssetInfo,
    holder: ChainAccount,
    recipient: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    require_min_tx_value!(get_value::<T>(amount)?);
    require!(
        liquidity::has_liquidity_to_reduce_asset::<T>(holder, asset, amount)?,
        Reason::InsufficientLiquidity
    );

    let holder_asset = AssetBalances::get(asset.asset, holder);
    let (holder_withdraw_amount, holder_borrow_amount) =
        withdraw_and_borrow_amount(holder_asset, amount)?;

    let holder_asset_new = sub_amount_from_balance(holder_asset, amount)?;
    let total_supply_new = sub_amount_from_raw(
        TotalSupplyAssets::get(asset.asset),
        holder_withdraw_amount,
        Reason::InsufficientTotalFunds,
    )?;
    let total_borrow_new =
        add_amount_to_raw(TotalBorrowAssets::get(asset.asset), holder_borrow_amount)?;

    let (cash_principal_post, last_index_post) = effect_of_asset_interest_internal(
        asset,
        holder,
        holder_asset,
        holder_asset_new,
        CashPrincipals::get(holder),
    )?;
    require!(
        total_borrow_new <= total_supply_new,
        Reason::InsufficientTotalFunds
    );

    LastIndices::insert(asset.asset, holder, last_index_post);
    CashPrincipals::insert(holder, cash_principal_post);
    TotalSupplyAssets::insert(asset.asset, total_supply_new);
    TotalBorrowAssets::insert(asset.asset, total_borrow_new);

    set_asset_balance_internal::<T>(asset.asset, holder, holder_asset_new);

    internal::notices::dispatch_extraction_notice::<T>(asset.asset, recipient, amount);

    <Module<T>>::deposit_event(Event::Extract(asset.asset, holder, recipient, amount.value));

    Ok(())
}

pub fn extract_cash_principal_internal<T: Config>(
    holder: ChainAccount,
    recipient: ChainAccount,
    principal: CashPrincipalAmount,
) -> Result<(), Reason> {
    let index = GlobalCashIndex::get();
    let amount = index.cash_quantity(principal)?;

    require_min_tx_value!(get_value::<T>(amount)?);
    require!(
        liquidity::has_liquidity_to_reduce_cash::<T>(holder, amount)?,
        Reason::InsufficientLiquidity
    );

    let holder_cash_principal = CashPrincipals::get(holder);
    let (_holder_withdraw_principal, holder_borrow_principal) =
        withdraw_and_borrow_principal(holder_cash_principal, principal)?;

    let chain_id = recipient.chain_id();
    let chain_cash_principal_new =
        add_principal_amounts(ChainCashPrincipals::get(chain_id), principal)?;
    let total_cash_principal_new =
        add_principal_amounts(TotalCashPrincipal::get(), holder_borrow_principal)?;
    let holder_cash_principal_new = holder_cash_principal.sub_amount(principal)?;

    ChainCashPrincipals::insert(chain_id, chain_cash_principal_new);
    CashPrincipals::insert(holder, holder_cash_principal_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    internal::notices::dispatch_cash_extraction_notice::<T>(recipient, principal);

    <Module<T>>::deposit_event(Event::ExtractCash(holder, recipient, principal, index));

    Ok(())
}

pub fn transfer_internal<T: Config>(
    asset: AssetInfo,
    sender: ChainAccount,
    recipient: ChainAccount,
    amount: AssetQuantity,
) -> Result<(), Reason> {
    let miner = get_some_miner::<T>();
    let index = GlobalCashIndex::get();
    let fee_principal = index.cash_principal_amount(TRANSFER_FEE)?;

    require_min_tx_value!(get_value::<T>(amount)?);

    CashPipeline::new()
        .transfer_asset::<T>(sender, recipient, asset.asset, amount)?
        .transfer_cash::<T>(sender, miner, fee_principal)?
        .check_collateralized::<T>(sender)?
        .commit::<T>();

    <Module<T>>::deposit_event(Event::Transfer(
        asset.asset,
        sender,
        recipient,
        amount.value,
    ));

    Ok(())
}

pub fn transfer_cash_principal_internal<T: Config>(
    sender: ChainAccount,
    recipient: ChainAccount,
    principal: CashPrincipalAmount,
) -> Result<(), Reason> {
    let miner = get_some_miner::<T>();
    let index = GlobalCashIndex::get();
    let fee_principal = index.cash_principal_amount(TRANSFER_FEE)?;
    let amount = index.cash_quantity(principal)?;

    require_min_tx_value!(get_value::<T>(amount)?);

    CashPipeline::new()
        .transfer_cash::<T>(sender, recipient, principal)?
        .transfer_cash::<T>(sender, miner, fee_principal)?
        .check_collateralized::<T>(sender)?
        .commit::<T>();

    <Module<T>>::deposit_event(Event::TransferCash(sender, recipient, principal, index));

    Ok(())
}

/// Calculates the current CASH principal of the account, including all interest from non-CASH markets.
pub fn get_cash_principal_with_asset_interest<T: Config>(
    account: ChainAccount,
) -> Result<CashPrincipal, Reason> {
    let mut principal = CashPrincipals::get(account);
    for (asset, _) in AssetsWithNonZeroBalance::iter_prefix(account) {
        let asset_info = get_asset::<T>(asset)?;
        let balance = AssetBalances::get(asset, account);
        (principal, _) =
            effect_of_asset_interest_internal(asset_info, account, balance, balance, principal)?;
    }
    Ok(principal)
}

/// Calculates the current total CASH value of the account, including all interest from non-CASH markets.
pub fn get_cash_balance_with_asset_interest<T: Config>(
    account: ChainAccount,
) -> Result<Balance, Reason> {
    Ok(GlobalCashIndex::get()
        .cash_balance(get_cash_principal_with_asset_interest::<T>(account)?)?)
}

// Asset Interest //

/// Return CASH Principal post asset interest, and updated asset index, for a given account.
pub fn effect_of_asset_interest_internal(
    asset_info: AssetInfo,
    account: ChainAccount,
    asset_balance_old: AssetBalance,
    asset_balance_new: AssetBalance,
    cash_principal_pre: CashPrincipal,
) -> Result<(CashPrincipal, AssetIndex), MathError> {
    let asset = asset_info.asset;
    let last_index = LastIndices::get(asset, account);
    let cash_index = if asset_balance_old >= 0 {
        SupplyIndices::get(asset)
    } else {
        BorrowIndices::get(asset)
    };
    let balance_old = asset_info.as_balance(asset_balance_old);
    let cash_principal_delta = cash_index.cash_principal_since(last_index, balance_old)?;
    let cash_principal_post = cash_principal_pre.add(cash_principal_delta)?;
    let last_index_post = if asset_balance_new >= 0 {
        SupplyIndices::get(asset)
    } else {
        BorrowIndices::get(asset)
    };
    Ok((cash_principal_post, last_index_post))
}

// Dispatch Extrinsic Lifecycle //

/// Block initialization wrapper.
// XXX we need to be able to mock Now (then get rid of this?)
pub fn on_initialize<T: Config>() -> Result<(), Reason> {
    on_initialize_internal::<T>(
        get_now::<T>(),
        LastYieldTimestamp::get(),
        LastBlockTimestamp::get(),
    )
}

/// Take a ratio of prices.
// If we generalized prices over the quote type, this would probably be:
//  Price<A, Q> / P<B, Q> -> Price<A, B>
// But we don't need generalize prices, just enough Decimals, so we have:
//  Price<A, { USD }> / Price<B, { USD }> -> Factor(B per A)
pub fn ratio(num: Price, denom: Price) -> Result<Factor, MathError> {
    Factor::from_fraction(num.value, denom.value)
}

/// Block initialization step that can fail.
pub fn on_initialize_internal<T: Config>(
    now: Timestamp,
    last_yield_timestamp: Timestamp,
    last_block_timestamp: Timestamp,
) -> Result<(), Reason> {
    // XXX re-evaluate how we do time, we don't really want this to be zero but there may
    // not actually be any good way to do "current" time per-se so what we have here is more like
    // the last block's time and the block before
    // XXX also we should try to inject Now (and previous) for tests instead of taking different path
    if now == 0 {
        return Err(Reason::TimeTravelNotAllowed);
    }
    if last_yield_timestamp == 0 || last_block_timestamp == 0 {
        // this is the first time we have seen a valid time, set it for LastYield and LastBlock
        if last_yield_timestamp == 0 {
            LastYieldTimestamp::put(now);
        }
        if last_block_timestamp == 0 {
            LastBlockTimestamp::put(now);
        }

        // All dt will be 0 so bail out here, no interest accrued in this block.
        return Ok(());
    }

    // Iterate through listed assets, summing the CASH principal they generated/paid last block
    let dt_since_last_block = now
        .checked_sub(last_block_timestamp)
        .ok_or(Reason::TimeTravelNotAllowed)?;
    let dt_since_last_yield = now
        .checked_sub(last_yield_timestamp)
        .ok_or(Reason::TimeTravelNotAllowed)?;
    let mut cash_principal_supply_increase = CashPrincipalAmount::ZERO;
    let mut cash_principal_borrow_increase = CashPrincipalAmount::ZERO;

    let last_block_cash_index = GlobalCashIndex::get();
    let last_yield_cash_index = LastYieldCashIndex::get();
    let cash_yield = CashYield::get();
    let price_cash = get_price_or_zero::<T>(CASH);

    let mut asset_updates: Vec<(ChainAsset, AssetIndex, AssetIndex)> = Vec::new();
    for (asset, asset_info) in SupportedAssets::iter() {
        let (asset_cost, asset_yield) = crate::core::get_rates::<T>(asset)?;
        let asset_units = asset_info.units();
        let price_asset = get_price_or_zero::<T>(asset_units);
        let price_ratio = ratio(price_asset, price_cash)?;
        let cash_borrow_principal_per_asset = last_block_cash_index
            .cash_principal_per_asset(asset_cost.simple(dt_since_last_block)?, price_ratio)?;
        let cash_hold_principal_per_asset = last_block_cash_index
            .cash_principal_per_asset(asset_yield.simple(dt_since_last_block)?, price_ratio)?;

        let supply_index = SupplyIndices::get(&asset);
        let borrow_index = BorrowIndices::get(&asset);
        let supply_index_new = supply_index.increment(cash_hold_principal_per_asset)?;
        let borrow_index_new = borrow_index.increment(cash_borrow_principal_per_asset)?;

        let supply_asset = Quantity::new(TotalSupplyAssets::get(asset), asset_units);
        let borrow_asset = Quantity::new(TotalBorrowAssets::get(asset), asset_units);
        cash_principal_supply_increase = cash_principal_supply_increase
            .add(cash_hold_principal_per_asset.cash_principal_amount(supply_asset)?)?;
        cash_principal_borrow_increase = cash_principal_borrow_increase
            .add(cash_borrow_principal_per_asset.cash_principal_amount(borrow_asset)?)?;

        asset_updates.push((asset.clone(), supply_index_new, borrow_index_new));
    }

    // Pay miners and update the CASH interest index on CASH itself
    if cash_yield == APR::ZERO {
        log!("Cash yield is zero. No interest earned on cash in this block.");
    }

    let total_cash_principal = TotalCashPrincipal::get();

    let increment = cash_yield.compound(dt_since_last_yield)?;
    if increment == CashIndex::ONE {
        log!("Index increment = 1. No interest on cash earned in this block!")
    }
    let cash_index_new = last_yield_cash_index.increment(increment)?; // XXX
    let total_cash_principal_new = total_cash_principal.add(cash_principal_borrow_increase)?;
    let miner_share_principal =
        cash_principal_borrow_increase.sub(cash_principal_supply_increase)?;

    let last_miner = get_some_miner::<T>(); // Miner not yet set for this block, so this is "last miner"
    let last_miner_share_principal = LastMinerSharePrincipal::get();
    let miner_cash_principal_old = CashPrincipals::get(&last_miner);
    let miner_cash_principal_new =
        miner_cash_principal_old.add_amount(last_miner_share_principal)?;

    // * BEGIN STORAGE ALL CHECKS AND FAILURES MUST HAPPEN ABOVE * //

    CashPrincipals::insert(last_miner, miner_cash_principal_new);
    log!(
        "Miner={:?} received {:?} principal for mining last block",
        String::from(last_miner),
        last_miner_share_principal
    );

    for (asset, new_supply_index, new_borrow_index) in asset_updates.drain(..) {
        SupplyIndices::insert(asset.clone(), new_supply_index);
        BorrowIndices::insert(asset, new_borrow_index);
    }

    GlobalCashIndex::put(cash_index_new);
    TotalCashPrincipal::put(total_cash_principal_new);
    LastMinerSharePrincipal::put(miner_share_principal);
    LastBlockTimestamp::put(now);

    // Possibly rotate in any scheduled next CASH rate
    if let Some((next_apr, next_start)) = CashYieldNext::get() {
        if next_start <= now {
            LastYieldTimestamp::put(next_start);
            LastYieldCashIndex::put(cash_index_new);
            CashYield::put(next_apr);
            CashYieldNext::kill();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn test_compute_cash_principal_per() -> Result<(), Reason> {
        // round numbers (unrealistic but very easy to check)
        let asset_rate = APR::from_nominal("0.30"); // 30% per year
        let dt = MILLISECONDS_PER_YEAR / 2; // for 6 months
        let cash_index = CashIndex::from_nominal("1.5"); // current index value 1.5
        let price_asset = Price::from_nominal(CASH.ticker, "1500"); // $1,500
        let price_cash = Price::from_nominal(CASH.ticker, "1");
        let price_ratio = ratio(price_asset, price_cash)?;

        let actual = cash_index.cash_principal_per_asset(asset_rate.simple(dt)?, price_ratio)?;
        let expected = CashPrincipalPerAsset::from_nominal("150"); // from hand calc
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_compute_cash_principal_per_specific_case() -> Result<(), Reason> {
        // a unit test related to previous unexpected larger scope test of on_initialize
        // this showed that we should divide by SECONDS_PER_YEAR last te prevent un-necessary truncation
        let asset_rate = APR::from_nominal("0.1225");
        let dt = MILLISECONDS_PER_YEAR / 4;
        let cash_index = CashIndex::from_nominal("1.123");
        let price_asset = Price::from_nominal(CASH.ticker, "1450");
        let price_cash = Price::from_nominal(CASH.ticker, "1");
        let price_ratio = ratio(price_asset, price_cash)?;

        let actual = cash_index.cash_principal_per_asset(asset_rate.simple(dt)?, price_ratio)?;
        let expected = CashPrincipalPerAsset::from_nominal("39.542520035618878005"); // from hand calc
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_compute_cash_principal_per_realistic_underflow_case() -> Result<(), Reason> {
        // a unit test related to previous unexpected larger scope test of on_initialize
        // This case showed that we should have more decimals on CASH token to avoid 0 interest
        // showing for common cases. We want "number go up" technology.
        let asset_rate = APR::from_nominal("0.156");
        let dt = 6000;
        let cash_index = CashIndex::from_nominal("4.629065392511782467");
        let price_asset = Price::from_nominal(CASH.ticker, "0.313242");
        let price_cash = Price::from_nominal(CASH.ticker, "1");
        let price_ratio = ratio(price_asset, price_cash)?;

        let actual = cash_index.cash_principal_per_asset(asset_rate.simple(dt)?, price_ratio)?;
        let expected = CashPrincipalPerAsset::from_nominal("0.000000002008426366"); // from hand calc
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_get_utilization() -> Result<(), Reason> {
        new_test_ext().execute_with(|| {
            initialize_storage();
            TotalSupplyAssets::insert(&Eth, 100);
            TotalBorrowAssets::insert(&Eth, 50);
            assert_eq!(
                crate::core::get_utilization::<Test>(Eth)?,
                Factor::from_nominal("0.5")
            );
            Ok(())
        })
    }

    #[test]
    fn test_get_borrow_rate() -> Result<(), Reason> {
        new_test_ext().execute_with(|| {
            initialize_storage();
            let kink_rate = 105;
            let asset = Eth;
            let asset_info = AssetInfo {
                rate_model: InterestRateModel::new_kink(
                    0,
                    kink_rate,
                    Factor::from_nominal("0.5"),
                    202,
                ),
                miner_shares: MinerShares::from_nominal("0.5"),
                ..AssetInfo::minimal(asset, ETH)
            };

            // 50% utilization and 50% miner shares
            SupportedAssets::insert(&asset, asset_info);
            TotalSupplyAssets::insert(&asset, 100);
            TotalBorrowAssets::insert(&asset, 50);

            assert_ok!(CashModule::set_rate_model(
                Origin::root(),
                asset,
                asset_info.rate_model
            ));

            let (borrow_rate, supply_rate) = crate::core::get_rates::<Test>(asset)?;
            assert_eq!(borrow_rate, kink_rate.into());
            assert_eq!(supply_rate, (kink_rate / 2 / 2).into());

            Ok(())
        })
    }

    #[test]
    fn test_get_assets() -> Result<(), Reason> {
        new_test_ext().execute_with(|| {
            initialize_storage();
            let assets = vec![
                AssetInfo {
                    liquidity_factor: FromStr::from_str("7890").unwrap(),
                    ..AssetInfo::minimal(
                        FromStr::from_str("eth:0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE")
                            .unwrap(),
                        FromStr::from_str("ETH/18").unwrap(),
                    )
                },
                AssetInfo {
                    ticker: FromStr::from_str("USD").unwrap(),
                    liquidity_factor: FromStr::from_str("7890").unwrap(),
                    ..AssetInfo::minimal(
                        FromStr::from_str("eth:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")
                            .unwrap(),
                        FromStr::from_str("USDC/6").unwrap(),
                    )
                },
            ];

            assert_eq!(crate::core::get_assets::<Test>()?, assets);

            Ok(())
        })
    }

    #[test]
    fn test_on_initialize() {
        new_test_ext().execute_with(|| {
            // XXX how to inject miner?
            let miner = ChainAccount::Eth([0; 20]);
            let asset = Eth;
            let asset_info = AssetInfo {
                rate_model: InterestRateModel::new_kink(0, 2500, Factor::from_nominal("0.5"), 5000),
                miner_shares: MinerShares::from_nominal("0.02"),
                ..AssetInfo::minimal(asset, ETH)
            };
            // XXX how to inject now / last yield timestamp?
            let last_yield_timestamp = 10;
            let now = last_yield_timestamp + MILLISECONDS_PER_YEAR / 4; // 3 months go by

            SupportedAssets::insert(&asset, asset_info);
            GlobalCashIndex::put(CashIndex::from_nominal("1.123"));
            LastYieldCashIndex::put(CashIndex::from_nominal("1.123"));
            SupplyIndices::insert(&asset, AssetIndex::from_nominal("1234"));
            BorrowIndices::insert(&asset, AssetIndex::from_nominal("1345"));
            TotalSupplyAssets::insert(asset.clone(), asset_info.as_quantity_nominal("300").value);
            TotalBorrowAssets::insert(asset.clone(), asset_info.as_quantity_nominal("150").value);
            CashYield::put(APR::from_nominal("0.24")); // 24% APR big number for easy to see interest
            TotalCashPrincipal::put(CashPrincipalAmount::from_nominal("450000")); // 450k cash principal
            CashPrincipals::insert(&miner, CashPrincipal::from_nominal("1"));
            pallet_oracle::Prices::insert(
                asset_info.ticker,
                1450_000000 as pallet_oracle::types::AssetPrice,
            ); // $1450 eth

            assert_ok!(
                on_initialize_internal::<Test>(now, last_yield_timestamp, last_yield_timestamp),
                ()
            );

            assert_eq!(
                SupplyIndices::get(&asset),
                AssetIndex::from_nominal("1273.542520035618878005")
            );
            assert_eq!(
                BorrowIndices::get(&asset),
                AssetIndex::from_nominal("1425.699020480854853072")
            );
            // note - the cash index number below is quite round due to the polynomial nature of
            // our approximation and the fact that the ratio in this case worked out to be a
            // base 10 number that terminates in that many digits.
            assert_eq!(
                GlobalCashIndex::get(),
                CashIndex::from_nominal("1.192441828000000000")
            );
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("462104.853072")
            );
            assert_eq!(
                CashPrincipals::get(&miner),
                CashPrincipal::from_nominal("1.000000")
            );
            // Run again to set miner true principal
            assert_ok!(
                on_initialize_internal::<Test>(now, last_yield_timestamp, last_yield_timestamp),
                ()
            );
            assert_eq!(
                CashPrincipals::get(&miner),
                CashPrincipal::from_nominal("243.097062")
            );
        });
    }

    #[test]
    fn test_on_initialize_next_yield_progression() {
        new_test_ext().execute_with(|| {
            let now = MILLISECONDS_PER_YEAR; // we are at year 1 (from epoc)
            let last_yield_timestamp = 3 * MILLISECONDS_PER_YEAR / 4; // last time we set the yield was 3 months ago
            let last_block_timestamp = now - 6 * 1000; // last block was 6 seconds ago
            let next_yield_timestamp = now + 6 * 1000; // we are going to set the next yield in the next block 6 seconds from "now"
            let global_cash_index_initial = CashIndex::from_nominal("1.123");
            let last_yield_cash_index_initial = CashIndex::from_nominal("1.111");
            let cash_yield_initial = APR::from_nominal("0.24");
            let cash_yield_next = APR::from_nominal("0.32");

            CashYield::put(cash_yield_initial);
            CashYieldNext::put((cash_yield_next, next_yield_timestamp));
            GlobalCashIndex::put(global_cash_index_initial);
            LastYieldCashIndex::put(last_yield_cash_index_initial);
            LastYieldTimestamp::put(last_yield_timestamp);

            assert_ok!(
                on_initialize_internal::<Test>(now, last_yield_timestamp, last_block_timestamp),
                ()
            );

            let increment_expected = cash_yield_initial
                .compound(now - last_yield_timestamp)
                .expect("could not compound interest during expected calc");
            let new_index_expected = last_yield_cash_index_initial
                .increment(increment_expected)
                .expect("could not increment index value during expected calc");
            let new_index_actual = GlobalCashIndex::get();
            assert_eq!(
                new_index_expected, new_index_actual,
                "current yield was not used to calculate the cash index increment"
            );
            let (next_yield_actual, next_yield_timestamp_actual) =
                CashYieldNext::get().expect("cash yield next was cleared unexpectedly");
            assert_eq!(
                next_yield_actual, cash_yield_next,
                "cash yield next yield was modified unexpectedly"
            );
            assert_eq!(
                next_yield_timestamp_actual, next_yield_timestamp,
                "cash yield next timestamp was modified unexpectedly"
            );
            assert_eq!(
                LastYieldTimestamp::get(),
                last_yield_timestamp,
                "last yield timestamp changed unexpectedly"
            );
            assert_eq!(
                LastYieldCashIndex::get(),
                last_yield_cash_index_initial,
                "last yield cash index was updated unexpectedly"
            );

            // simulate "a block goes by" in time
            let last_block_timestamp = now;
            let now = next_yield_timestamp;

            assert_ok!(
                on_initialize_internal::<Test>(now, last_yield_timestamp, last_block_timestamp),
                ()
            );

            let increment_expected = cash_yield_initial
                .compound(now - last_yield_timestamp)
                .expect("could not compound interest during expected calc");
            let new_index_expected = last_yield_cash_index_initial
                .increment(increment_expected)
                .expect("could not increment index value during expected calc");
            let new_index_actual = GlobalCashIndex::get();
            assert_eq!(
                new_index_expected, new_index_actual,
                "current yield was not used to calculate the cash index increment"
            );
            assert!(CashYieldNext::get().is_none(), "next yield was not cleared");
            let actual_cash_yield = CashYield::get();
            assert_eq!(
                actual_cash_yield, cash_yield_next,
                "the current yield was not updated to next yield"
            );
            assert_eq!(
                LastYieldTimestamp::get(),
                next_yield_timestamp,
                "last yield timestamp was not updated"
            );
            assert_eq!(
                LastYieldCashIndex::get(),
                new_index_actual,
                "last yield cash index was not updated correctly"
            );

            // simulate "a block goes by" in time
            let last_block_timestamp = now;
            let now = now + 6 * 1000;
            let new_cash_index_baseline = new_index_actual;

            assert_ok!(
                on_initialize_internal::<Test>(now, next_yield_timestamp, last_block_timestamp),
                ()
            );

            let increment_expected = cash_yield_next
                .compound(now - next_yield_timestamp)
                .expect("could not compound interest during expected calc");
            let new_index_expected = new_cash_index_baseline
                .increment(increment_expected)
                .expect("could not increment index value during expected calc");
            let new_index_actual = GlobalCashIndex::get();
            assert_eq!(
                new_index_expected, new_index_actual,
                "current yield was not used to calculate the cash index increment"
            );
            assert!(CashYieldNext::get().is_none(), "next yield was not cleared");
            let actual_cash_yield = CashYield::get();
            assert_eq!(
                actual_cash_yield, cash_yield_next,
                "the current yield was not updated to next yield"
            );
            assert_eq!(
                LastYieldTimestamp::get(),
                next_yield_timestamp,
                "last yield timestamp was modified unexpectedly"
            );
            assert_eq!(
                LastYieldCashIndex::get(),
                new_cash_index_baseline,
                "last yield cash index was updated unexpectedly"
            );
        });
    }

    #[test]
    fn test_now() {
        new_test_ext().execute_with(|| {
            let expected = 123;
            <pallet_timestamp::Module<Test>>::set_timestamp(expected);
            let actual = get_now::<Test>();
            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn test_set_asset_balance_internal() {
        new_test_ext().execute_with(|| {
            let asset1 = ChainAsset::Eth([1; 20]);
            let asset2 = ChainAsset::Eth([2; 20]);
            let asset3 = ChainAsset::Eth([3; 20]);
            let account = ChainAccount::Eth([20; 20]);

            let nonzero_balance = 1;
            let zero_balance = 0;
            // asset1 and asset2 both have nonzero balances
            AssetBalances::insert(asset1, account, nonzero_balance);
            AssetBalances::insert(asset2, account, nonzero_balance);
            AssetsWithNonZeroBalance::insert(account, asset1, ());
            AssetsWithNonZeroBalance::insert(account, asset2, ());

            set_asset_balance_internal::<Test>(asset1, account, zero_balance);
            assert!(
                !AssetsWithNonZeroBalance::contains_key(account, asset1),
                "set to zero should be zeroed out"
            );
            assert!(
                AssetsWithNonZeroBalance::contains_key(account, asset2),
                "should not be zeroed out"
            );
            assert_eq!(AssetBalances::get(asset1, account), zero_balance);
            assert_eq!(AssetBalances::get(asset2, account), nonzero_balance);

            set_asset_balance_internal::<Test>(asset3, account, nonzero_balance);
            assert!(
                !AssetsWithNonZeroBalance::contains_key(account, asset1),
                "set to zero should be zeroed out"
            );
            assert!(
                AssetsWithNonZeroBalance::contains_key(account, asset2),
                "should not be zeroed out"
            );
            assert!(
                AssetsWithNonZeroBalance::contains_key(account, asset3),
                "should not be zeroed out"
            );
            assert_eq!(AssetBalances::get(asset1, account), zero_balance);
            assert_eq!(AssetBalances::get(asset2, account), nonzero_balance);
            assert_eq!(AssetBalances::get(asset3, account), nonzero_balance);
        });
    }
}
