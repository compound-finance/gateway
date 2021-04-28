use crate::{
    chains::ChainAsset,
    rates::{InterestRateModel, APR},
    reason::Reason,
    types::{
        AssetAmount, AssetInfo, AssetQuantity, CashPrincipalAmount, Factor, LiquidityFactor,
        Quantity, USDQuantity, Units,
    },
    Config, Event, GlobalCashIndex, Module, SupportedAssets, TotalBorrowAssets, TotalSupplyAssets,
};
use frame_support::storage::{IterableStorageMap, StorageMap, StorageValue};
use pallet_oracle::types::Price;

/// Set the liquidity factor for a supported asset.
pub fn set_liquidity_factor<T: Config>(
    asset: ChainAsset,
    factor: LiquidityFactor,
) -> Result<(), Reason> {
    let asset_info = get_asset::<T>(asset)?;
    support_asset::<T>(AssetInfo {
        liquidity_factor: factor,
        ..asset_info
    })
}

/// Set the rate model for a supported asset.
pub fn set_rate_model<T: Config>(
    asset: ChainAsset,
    model: InterestRateModel,
) -> Result<(), Reason> {
    let asset_info = get_asset::<T>(asset)?;
    model.check_parameters()?;
    support_asset::<T>(AssetInfo {
        rate_model: model,
        ..asset_info
    })
}

/// Support an asset by defining its metadata.
pub fn support_asset<T: Config>(asset_info: AssetInfo) -> Result<(), Reason> {
    SupportedAssets::insert(&asset_info.asset, asset_info);
    <Module<T>>::deposit_event(Event::AssetModified(asset_info));
    Ok(())
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

#[cfg(test)]
mod tests {
    use crate::{
        rates::*,
        tests::{assert_ok, assets::*, common::*, mock::*, *},
        types::*,
        *,
    };

    #[test]
    fn test_set_liquidity_factor_not_supported() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                super::set_liquidity_factor::<Test>(Eth, Factor::from_nominal("0.8")),
                Err(Reason::AssetNotSupported)
            );
        });
    }

    #[test]
    fn test_set_liquidity_factor_supported() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(super::set_liquidity_factor::<Test>(
                Eth,
                Factor::from_nominal("0.8")
            ));
            assert_eq!(
                SupportedAssets::get(Eth).unwrap().liquidity_factor,
                Factor::from_nominal("0.8")
            );
        });
    }

    #[test]
    fn test_set_rate_model_not_supported() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                super::set_rate_model::<Test>(
                    Eth,
                    InterestRateModel::Fixed {
                        rate: APR::from_nominal("0.10")
                    }
                ),
                Err(Reason::AssetNotSupported)
            );
        });
    }

    #[test]
    fn test_set_rate_model_supported() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(super::set_rate_model::<Test>(
                Eth,
                InterestRateModel::Fixed {
                    rate: APR::from_nominal("0.10")
                }
            ));
            assert_eq!(
                SupportedAssets::get(Eth).unwrap().rate_model,
                InterestRateModel::Fixed {
                    rate: APR::from_nominal("0.10")
                }
            );
        });
    }

    #[test]
    fn test_support_asset() {
        new_test_ext().execute_with(|| {
            assert_ok!(super::support_asset::<Test>(eth));
            assert_eq!(SupportedAssets::get(Eth), Some(eth));

            let events_post: Vec<_> = System::events().into_iter().collect();
            let asset_modified_event = events_post.into_iter().next().unwrap();
            assert_eq!(
                mock::Event::pallet_cash(crate::Event::AssetModified(eth)),
                asset_modified_event.event
            );
        })
    }

    #[test]
    fn test_support_asset_again() {
        new_test_ext().execute_with(|| {
            assert_ok!(super::support_asset::<Test>(eth));
            assert_ok!(super::support_asset::<Test>(eth));
            assert_eq!(SupportedAssets::get(Eth), Some(eth));
        })
    }

    #[test]
    fn test_get_utilization() -> Result<(), Reason> {
        new_test_ext().execute_with(|| {
            initialize_storage();
            TotalSupplyAssets::insert(&Eth, 100);
            TotalBorrowAssets::insert(&Eth, 50);
            assert_eq!(
                super::get_utilization::<Test>(Eth)?,
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

            let (borrow_rate, supply_rate) = super::get_rates::<Test>(asset)?;
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

            assert_eq!(super::get_assets::<Test>()?, assets);

            Ok(())
        })
    }
}
