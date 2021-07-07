use crate::{
    chains::ChainAccount,
    internal,
    params::MIN_TX_VALUE,
    pipeline::CashPipeline,
    reason::Reason,
    require, require_min_tx_value,
    types::{AssetInfo, AssetQuantity, CashIndex, CashPrincipalAmount},
    Config, Event, GlobalCashIndex, Module,
};
use frame_support::storage::StorageValue;

pub fn extract_internal<T: Config>(
    asset: AssetInfo,
    sender: ChainAccount,
    recipient: ChainAccount,
    quantity: AssetQuantity,
) -> Result<(), Reason> {
    require_min_tx_value!(internal::assets::get_value::<T>(quantity)?);

    CashPipeline::new()
        .extract_asset::<T>(sender, asset.asset, quantity)?
        .check_collateralized::<T>(sender)?
        .check_sufficient_total_funds::<T>(asset)?
        .commit::<T>();

    internal::notices::dispatch_extraction_notice::<T>(asset.asset, recipient, quantity);

    <Module<T>>::deposit_event(Event::Extract(
        asset.asset,
        sender,
        recipient,
        quantity.value,
    ));

    Ok(())
}

pub fn extract_cash_principal_internal<T: Config>(
    sender: ChainAccount,
    recipient: ChainAccount,
    principal: CashPrincipalAmount,
) -> Result<(), Reason> {
    let index: CashIndex = GlobalCashIndex::get();
    let amount = index.cash_quantity(principal)?;
    require_min_tx_value!(internal::assets::get_value::<T>(amount)?);

    CashPipeline::new()
        .extract_cash::<T>(sender, principal)?
        .check_collateralized::<T>(sender)?
        .commit::<T>();

    internal::notices::dispatch_cash_extraction_notice::<T>(recipient, principal);

    <Module<T>>::deposit_event(Event::ExtractCash(sender, recipient, principal, index));

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{internal::assets::*, tests::*};
    use pallet_oracle::{types::Price, Prices};

    #[test]
    fn test_extract_asset_without_supply() -> Result<(), Reason> {
        let jared = ChainAccount::from_str("Eth:0x18c8F1222083997405F2E482338A4650ac02e1d6")?;
        let max = ChainAccount::from_str("Eth:0x7f89077b122afaaf6ab50aa12e9cb46bb9a058c4")?;

        new_test_ext().execute_with(|| {
            Prices::insert(ETH.ticker, Price::from_nominal(ETH.ticker, "2000.19").value);
            SupportedAssets::insert(&Eth, eth);
            CashPrincipals::insert(&jared, CashPrincipal::from_nominal("10000"));

            assert_eq!(TotalSupplyAssets::get(&Eth), 0);
            assert_err!(
                super::extract_internal::<Test>(eth, jared, max, qty!("1", ETH)),
                Reason::InsufficientTotalFunds
            );

            Ok(())
        })
    }

    #[test]
    fn test_extract_internal_min_value() -> Result<(), Reason> {
        let asset = ChainAsset::Eth([238; 20]);
        let asset_info = AssetInfo::minimal(asset, ETH);
        let holder = ChainAccount::Eth([0; 20]);
        let recipient = ChainAccount::Eth([0; 20]);

        new_test_ext().execute_with(|| {
            SupportedAssets::insert(&asset, asset_info);
            Prices::insert(asset_info.ticker, 100_000); // $0.10
            let quantity = get_quantity::<Test>(asset, 5_000_000_000_000_000_000).unwrap();
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

            assert_err!(
                super::extract_internal::<Test>(asset_info, holder, recipient, quantity),
                Reason::MinTxValueNotMet
            );

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

            Ok(())
        })
    }

    #[test]
    fn test_extract_internal_sufficient_value() -> Result<(), Reason> {
        let eth_asset = [238; 20];
        let asset = ChainAsset::Eth(eth_asset);
        let asset_info = AssetInfo {
            liquidity_factor: LiquidityFactor::from_nominal("1"),
            ..AssetInfo::minimal(asset, ETH)
        };
        let eth_holder = [0; 20];
        let eth_recipient = [0; 20];
        let holder = ChainAccount::Eth(eth_holder);
        let recipient = ChainAccount::Eth(eth_recipient);

        new_test_ext().execute_with(|| {
            SupportedAssets::insert(&asset, asset_info);
            Prices::insert(asset_info.ticker, 100_000); // $0.10
            let quantity = get_quantity::<Test>(asset, 50_000_000_000_000_000_000).unwrap();
            let hodl_balance = quantity.value * 5;
            AssetBalances::insert(asset, holder, hodl_balance as AssetBalance);
            AssetsWithNonZeroBalance::insert(holder, asset, ());
            TotalSupplyAssets::insert(&asset, hodl_balance);

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

            assert_ok!(super::extract_internal::<Test>(
                asset_info, holder, recipient, quantity
            ));

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
            assert_eq!(total_supply_pre - 50000000000000000000, total_supply_post);
            assert_eq!(total_borrows_pre, total_borrows_post);
            assert_eq!(events_pre.len() + 2, events_post.len());

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
                mocks::Event::pallet_cash(crate::Event::Notice(
                    expected_notice_id,
                    expected_notice,
                    expected_notice_encoded
                )),
                notice_event.event
            );

            Ok(())
        })
    }

    #[test]
    fn test_extract_internal_notice_ids() -> Result<(), Reason> {
        let eth_asset = [238; 20];
        let asset = ChainAsset::Eth(eth_asset);
        let asset_info = AssetInfo {
            liquidity_factor: LiquidityFactor::from_nominal("1"),
            ..AssetInfo::minimal(asset, ETH)
        };
        let eth_holder = [0; 20];
        let eth_recipient = [0; 20];
        let holder = ChainAccount::Eth(eth_holder);
        let recipient = ChainAccount::Eth(eth_recipient);

        new_test_ext().execute_with(|| {
            SupportedAssets::insert(&asset, asset_info);
            Prices::insert(asset_info.ticker, 100_000); // $0.10
            let quantity = get_quantity::<Test>(asset, 50_000_000_000_000_000_000).unwrap();
            let hodl_balance = quantity.value * 5;
            AssetBalances::insert(asset, holder, hodl_balance as AssetBalance);
            AssetsWithNonZeroBalance::insert(holder, asset, ());
            TotalSupplyAssets::insert(&asset, hodl_balance);

            let notices_pre: Vec<(NoticeId, Notice)> = Notices::iter_prefix(ChainId::Eth).collect();
            let notice_states_pre: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let latest_notice_pre: Option<(NoticeId, ChainHash)> = LatestNotice::get(ChainId::Eth);
            let notice_hashes_pre: Vec<(ChainHash, NoticeId)> = NoticeHashes::iter().collect();
            let account_notices_pre: Vec<(ChainAccount, Vec<NoticeId>)> =
                AccountNotices::iter().collect();

            assert_eq!(LatestNotice::get(ChainId::Eth), None);
            assert_ok!(super::extract_internal::<Test>(
                asset_info, holder, recipient, quantity
            ));

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
            assert_ok!(super::extract_internal::<Test>(
                asset_info, holder, recipient, quantity
            ));

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

            Ok(())
        })
    }
}
