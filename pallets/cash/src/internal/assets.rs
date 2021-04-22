use crate::{
    chains::ChainAsset,
    core::get_asset,
    rates::InterestRateModel,
    reason::Reason,
    types::{AssetInfo, LiquidityFactor},
    Config, SupportedAssets,
};
use frame_support::storage::StorageMap;

pub fn set_liquidity_factor<T: Config>(
    asset: ChainAsset,
    factor: LiquidityFactor,
) -> Result<(), Reason> {
    let asset_info = get_asset::<T>(asset)?;
    SupportedAssets::insert(
        &asset,
        AssetInfo {
            liquidity_factor: factor,
            ..asset_info
        },
    );
    Ok(())
}

pub fn set_rate_model<T: Config>(
    asset: ChainAsset,
    model: InterestRateModel,
) -> Result<(), Reason> {
    let asset_info = get_asset::<T>(asset)?;
    model.check_parameters()?;
    SupportedAssets::insert(
        &asset,
        AssetInfo {
            rate_model: model,
            ..asset_info
        },
    );
    Ok(())
}

pub fn support_asset<T: Config>(asset: ChainAsset, asset_info: AssetInfo) -> Result<(), Reason> {
    // TODO: Should we perform sanity checks here?
    SupportedAssets::insert(&asset, asset_info);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rates::*,
        tests::{assert_ok, assets::*, common::*, mock::*},
        types::*,
        *,
    };

    #[test]
    fn test_set_liquidity_factor_not_supported() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                set_liquidity_factor::<Test>(Eth, Factor::from_nominal("0.8")),
                Err(Reason::AssetNotSupported)
            );
        });
    }

    #[test]
    fn test_set_liquidity_factor_supported() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(set_liquidity_factor::<Test>(
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
                set_rate_model::<Test>(
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
            assert_ok!(set_rate_model::<Test>(
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
            assert_ok!(support_asset::<Test>(Eth, eth));
            assert_eq!(SupportedAssets::get(Eth), Some(eth));
        })
    }

    #[test]
    fn test_support_asset_again() {
        new_test_ext().execute_with(|| {
            assert_ok!(support_asset::<Test>(Eth, eth));
            assert_ok!(support_asset::<Test>(Eth, eth));
            assert_eq!(SupportedAssets::get(Eth), Some(eth));
        })
    }
}
