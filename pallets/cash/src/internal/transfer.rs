// Note: The substrate build requires these be re-exported.
use frame_support::storage::StorageValue;

// Import these traits so we can interact with the substrate storage modules.
use crate::{
    chains::ChainAccount,
    core::{get_some_miner, get_value},
    params::{MIN_TX_VALUE, TRANSFER_FEE},
    pipeline::CashPipeline,
    reason::Reason,
    require, require_min_tx_value,
    types::{AssetInfo, AssetQuantity, CashPrincipalAmount},
    Config, Event, GlobalCashIndex, Module,
};

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
