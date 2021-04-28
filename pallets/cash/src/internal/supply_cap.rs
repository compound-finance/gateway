use crate::{
    chains::ChainAsset,
    internal,
    reason::Reason,
    types::{AssetAmount, AssetInfo},
    Config,
};

pub fn set_supply_cap<T: Config>(asset: ChainAsset, cap: AssetAmount) -> Result<(), Reason> {
    let asset_info = internal::assets::get_asset::<T>(asset)?;
    internal::assets::support_asset::<T>(AssetInfo {
        supply_cap: cap,
        ..asset_info
    })?;

    internal::notices::dispatch_supply_cap_notice::<T>(asset, cap);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chains::*, notices::*, tests::*, types::*, LatestNotice, NoticeStates, Notices};
    use frame_support::storage::{IterableStorageDoubleMap, StorageDoubleMap, StorageMap};

    #[test]
    fn test_set_supply_cap_not_supported() {
        new_test_ext().execute_with(|| {
            let asset = ChainAsset::Eth([100; 20]);
            assert_eq!(SupportedAssets::get(asset), None);
            assert_eq!(
                set_supply_cap::<Test>(asset, 1000),
                Err(Reason::AssetNotSupported)
            );
            // TODO: Check event and/or notice?
            assert_eq!(SupportedAssets::get(asset), None);
        });
    }

    #[test]
    fn test_set_supply_cap_supported() {
        new_test_ext().execute_with(|| {
            let asset = ChainAsset::Eth([100; 20]);
            let asset_info = AssetInfo::minimal(asset, USD);
            let new_asset_info = AssetInfo {
                supply_cap: 1000,
                ..asset_info
            };
            let events_pre: Vec<_> = System::events().into_iter().collect();
            SupportedAssets::insert(asset, asset_info);
            assert_eq!(SupportedAssets::get(asset), Some(asset_info));
            assert_eq!(set_supply_cap::<Test>(asset, 1000), Ok(()));
            assert_eq!(SupportedAssets::get(asset), Some(new_asset_info));
            let events_post: Vec<_> = System::events().into_iter().collect();

            assert_eq!(events_pre.len() + 2, events_post.len());

            let set_supply_cap_event = events_post.into_iter().next().unwrap();

            assert_eq!(
                mock::Event::pallet_cash(crate::Event::AssetModified(new_asset_info)),
                set_supply_cap_event.event
            );

            let notice_state_post: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let notice_state = notice_state_post.into_iter().next().unwrap();
            let notice = Notices::get(notice_state.0, notice_state.1);

            // bumps era
            let expected_notice_id = NoticeId(1, 0);
            let expected_notice = Notice::SetSupplyCapNotice(SetSupplyCapNotice::Eth {
                id: expected_notice_id,
                parent: [0u8; 32],
                asset: [100; 20],
                cap: 1000,
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
                Some((expected_notice_id, expected_notice.hash()))
            );
        });
    }
}
