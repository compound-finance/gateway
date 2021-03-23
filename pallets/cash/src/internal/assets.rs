use frame_support::storage::StorageMap;

use crate::{
    chains::ChainAsset,
    core::get_asset,
    rates::InterestRateModel,
    reason::Reason,
    types::{AssetInfo, LiquidityFactor},
    Config, SupportedAssets,
};

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

pub fn support_asset<T: Config>(asset_info: AssetInfo) -> Result<(), Reason> {
    SupportedAssets::insert(&asset_info.asset, asset_info);
    Ok(())
}
