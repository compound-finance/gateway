// Note: The substrate build requires these be re-exported.
pub use our_std::{
    cmp::{max, min},
    convert::TryFrom,
    fmt, result,
    result::Result,
};

// Import these traits so we can interact with the substrate storage modules.
use frame_support::storage::{IterableStorageMap, StorageDoubleMap, StorageMap, StorageValue};

use crate::{
    chains::{
        eth, ChainAccount, ChainAsset, ChainAssetAccount, ChainSignature, ChainSignatureList,
    },
    notices,
    notices::{EncodeNotice, ExtractionNotice, Notice, NoticeId, NoticeStatus},
    params::MIN_TX_VALUE,
    reason::{MathError, Reason},
    symbol::{Symbol, CASH},
    types::{
        AssetAmount, AssetBalance, AssetQuantity, CashPrincipal, CashQuantity, Int, Price,
        Quantity, SafeMul, USDQuantity,
    },
    AssetBalances, AssetSymbols, Call, CashPrincipals, ChainCashPrincipals, Config, Event,
    GlobalCashIndex, Module, NoticeQueue, Prices, SubmitTransaction, TotalBorrowAssets,
    TotalCashPrincipal, TotalSupplyAssets,
};

use sp_runtime::print;

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

pub fn next_notice_id() -> Result<NoticeId, Reason> {
    let notice_id = NoticeId(0, 0); // XXX need to keep state of current gen/within gen for each, also parent
                                    // TODO: This asset needs to match the chain-- which for Cash, there might be different
                                    //       versions of a given asset per chain

    Ok(notice_id)
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

pub fn passes_validation_threshold(signers_len: u8, validators_len: u8) -> bool {
    signers_len > validators_len * 2 / 3
}

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

    // TODO: Significantly improve this
    let notice_id = next_notice_id()?;
    let notice = Notice::ExtractionNotice(match chain_asset_account {
        ChainAssetAccount::Eth(eth_asset, eth_account) => Ok(ExtractionNotice::Eth {
            id: notice_id,
            parent: [0u8; 32],
            asset: eth_asset,
            account: eth_account,
            amount: amount.1,
        }),
        _ => Err(Reason::AssetExtractionNotSupported),
    }?);
    let encoded_notice = notice.encode_notice();

    // Add to Notice Queue
    NoticeQueue::insert(notice_id, NoticeStatus::from(notice.clone()));

    // Deposit Notice Event
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
    for (notice_id, notice_status) in NoticeQueue::iter() {
        match notice_status {
            NoticeStatus::Pending {
                signature_pairs,
                notice,
            } => {
                let signer = notices::get_signer_key_for_notice(&notice)?;

                if !notices::has_signer(&signature_pairs, signer) {
                    // find parent
                    // id = notice.gen_id(parent)

                    // XXX
                    // submit onchain call for aggregating the price
                    let signature: ChainSignature = notices::sign_notice_chain(&notice)?;

                    print(
                        format!("Posting Signature for [{},{}]", notice_id.0, notice_id.1).as_str(),
                    );

                    let call = <Call<T>>::publish_signature(notice_id, signature);

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
    notice_id: NoticeId,
    signature: ChainSignature,
) -> Result<(), Reason> {
    print(format!("Publishing Signature: [{},{}]", notice_id.0, notice_id.1).as_str());
    let status = <NoticeQueue>::get(notice_id).unwrap_or(NoticeStatus::Missing);

    match status {
        NoticeStatus::Missing => Ok(()),

        NoticeStatus::Pending {
            signature_pairs,
            notice,
        } => {
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
            // NoticeQueue::insert(notice_id, NoticeStatus::Done);
            // Self::deposit_event(Event::SignedNotice(ChainId::Eth, notice_id, notice.encode_notice(), signatures)); // XXX generic
            // Ok(())
            // } else {
            NoticeQueue::insert(
                notice_id,
                NoticeStatus::Pending {
                    signature_pairs: signature_pairs_next,
                    notice,
                },
            );
            Ok(())
            // }
        }

        NoticeStatus::Done => {
            // TODO: Eventually we should be deleting here (based on monotonic ids and signing order)
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::*;
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

    #[test]
    fn test_next_notice_id() {
        assert_eq!(next_notice_id(), Ok(NoticeId(0, 0)));
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
            let notice_queue_pre: Vec<(NoticeId, NoticeStatus)> = NoticeQueue::iter().collect();
            let events_pre: Vec<_> = System::events().into_iter().collect();

            let res = extract_internal::<Test>(asset, holder, recipient, quantity);

            assert_eq!(res, Err(Reason::MinTxValueNotMet));

            let asset_balances_post = AssetBalances::get(asset, holder);
            let total_supply_post = TotalSupplyAssets::get(asset);
            let total_borrows_post = TotalBorrowAssets::get(asset);
            let notice_queue_post: Vec<(NoticeId, NoticeStatus)> = NoticeQueue::iter().collect();
            let events_post: Vec<_> = System::events().into_iter().collect();

            assert_eq!(asset_balances_pre, asset_balances_post);
            assert_eq!(total_supply_pre, total_supply_post);
            assert_eq!(total_borrows_pre, total_borrows_post);
            assert_eq!(notice_queue_pre.len(), notice_queue_post.len());
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
            let notice_queue_pre: Vec<(NoticeId, NoticeStatus)> = NoticeQueue::iter().collect();
            let events_pre: Vec<_> = System::events().into_iter().collect();

            let res = extract_internal::<Test>(asset, holder, recipient, quantity);

            assert_eq!(res, Ok(()));

            let asset_balances_post = AssetBalances::get(asset, holder);
            let total_supply_post = TotalSupplyAssets::get(asset);
            let total_borrows_post = TotalBorrowAssets::get(asset);
            let notice_queue_post: Vec<(NoticeId, NoticeStatus)> = NoticeQueue::iter().collect();
            let events_post: Vec<_> = System::events().into_iter().collect();

            assert_eq!(
                asset_balances_pre - 50000000000000000000,
                asset_balances_post
            ); // 50e18
            assert_eq!(total_supply_pre, total_supply_post);
            assert_eq!(total_borrows_pre + 50000000000000000000, total_borrows_post);
            assert_eq!(notice_queue_pre.len() + 1, notice_queue_post.len());
            assert_eq!(events_pre.len() + 1, events_post.len());

            let notice_status = notice_queue_post.into_iter().next().unwrap();
            let notice_event = events_post.into_iter().next().unwrap();

            let expected_notice_id = NoticeId(0, 0);
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
                    expected_notice_id,
                    NoticeStatus::Pending {
                        signature_pairs: ChainSignatureList::Eth(vec![]),
                        notice: expected_notice.clone()
                    }
                ),
                notice_status
            );

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
}
