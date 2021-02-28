use frame_support::storage::StorageMap;

use crate::{
    chains::{ChainAsset, ChainHash},
    core::dispatch_notice_internal,
    notices::{Notice, SetSupplyCapNotice},
    reason::Reason,
    require,
    types::{AssetAmount, AssetInfo},
    Config, Event, Module, SupportedAssets,
};

pub fn set_supply_cap<T: Config>(chain_asset: ChainAsset, cap: AssetAmount) -> Result<(), Reason> {
    require!(
        SupportedAssets::contains_key(chain_asset),
        Reason::AssetNotSupported
    );

    SupportedAssets::mutate(chain_asset, |maybe_asset_info| {
        if let Some(asset_info) = maybe_asset_info {
            asset_info.supply_cap = cap;
        }
        *maybe_asset_info
    });

    <Module<T>>::deposit_event(Event::SetSupplyCap(chain_asset, cap));

    // XXX fix me this cannot fail
    dispatch_notice_internal::<T>(chain_asset.chain_id(), None, &|notice_id, parent_hash| {
        Ok(Notice::SetSupplyCapNotice(
            match (chain_asset, parent_hash) {
                (ChainAsset::Eth(eth_asset), ChainHash::Eth(eth_parent_hash)) => {
                    Ok(SetSupplyCapNotice::Eth {
                        id: notice_id,
                        parent: eth_parent_hash,
                        asset: eth_asset,
                        cap: cap,
                    })
                }
                _ => Err(Reason::AssetNotSupported),
            }?,
        ))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::{ChainId, ChainSignatureList},
        mock,
        mock::*,
        notices::{NoticeId, NoticeState},
        symbol::USD,
        LatestNotice, NoticeStates, Notices,
    };
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
            let events_pre: Vec<_> = System::events().into_iter().collect();
            SupportedAssets::insert(asset, asset_info);
            assert_eq!(SupportedAssets::get(asset), Some(asset_info));
            assert_eq!(set_supply_cap::<Test>(asset, 1000), Ok(()));
            assert_eq!(
                SupportedAssets::get(asset),
                Some(AssetInfo {
                    supply_cap: 1000,
                    ..asset_info
                })
            );
            let events_post: Vec<_> = System::events().into_iter().collect();

            assert_eq!(events_pre.len() + 2, events_post.len());

            let set_supply_cap_event = events_post.into_iter().next().unwrap();

            assert_eq!(
                mock::Event::pallet_cash(crate::Event::SetSupplyCap(asset, 1000)),
                set_supply_cap_event.event
            );

            let notice_state_post: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let notice_state = notice_state_post.into_iter().next().unwrap();
            let notice = Notices::get(notice_state.0, notice_state.1);

            let expected_notice_id = NoticeId(0, 1);
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
                Some((NoticeId(0, 1), expected_notice.hash()))
            );
        });
    }
}
