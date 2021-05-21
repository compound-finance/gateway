use crate::{
    chains::ChainAccount,
    internal::{assets::get_value, miner::get_some_miner},
    params::{MIN_TX_VALUE, TRANSFER_FEE},
    pipeline::CashPipeline,
    reason::Reason,
    require, require_min_tx_value,
    types::{AssetInfo, AssetQuantity, CashPrincipalAmount},
    Config, Event, GlobalCashIndex, Module,
};
use frame_support::storage::StorageValue;

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
    <Module<T>>::deposit_event(Event::TransferCash(sender, miner, fee_principal, index));
    <Module<T>>::deposit_event(Event::MinerPaid(miner, fee_principal));

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
    <Module<T>>::deposit_event(Event::TransferCash(sender, miner, fee_principal, index));
    <Module<T>>::deposit_event(Event::MinerPaid(miner, fee_principal));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        tests::{assets::*, common::*, mock::*},
        types::*,
        *,
    };

    #[allow(non_upper_case_globals)]
    const miner: ChainAccount = ChainAccount::Eth([0u8; 20]);
    #[allow(non_upper_case_globals)]
    const account_a: ChainAccount = ChainAccount::Eth([1u8; 20]);
    #[allow(non_upper_case_globals)]
    const account_b: ChainAccount = ChainAccount::Eth([2u8; 20]);

    #[test]
    fn test_transfer_internal_unsupported() {
        new_test_ext().execute_with(|| {
            let amount: AssetQuantity = usdc.as_quantity_nominal("1");

            assert_eq!(
                transfer_internal::<Test>(usdc, account_a, account_b, amount),
                Err(Reason::AssetNotSupported),
            );
        });
    }

    #[test]
    fn test_transfer_internal_below_min() {
        new_test_ext().execute_with(|| {
            init_usdc_asset().unwrap();
            let amount: AssetQuantity = usdc.as_quantity_nominal("0.1");

            assert_eq!(
                transfer_internal::<Test>(usdc, account_a, account_b, amount),
                Err(Reason::MinTxValueNotMet),
            );
        });
    }

    #[test]
    fn test_transfer_internal_undercollateralized() {
        new_test_ext().execute_with(|| {
            init_usdc_asset().unwrap();
            let amount: AssetQuantity = usdc.as_quantity_nominal("1");

            assert_eq!(
                transfer_internal::<Test>(usdc, account_a, account_b, amount),
                Err(Reason::InsufficientLiquidity),
            );
        });
    }

    #[test]
    fn test_transfer_internal_undercollateralized_for_fee() {
        new_test_ext().execute_with(|| {
            init_usdc_asset().unwrap();
            let amount: AssetQuantity = usdc.as_quantity_nominal("1");

            init_asset_balance(Usdc, account_a, Balance::from_nominal("1", USD).value);

            assert_eq!(
                transfer_internal::<Test>(usdc, account_a, account_b, amount),
                Err(Reason::InsufficientLiquidity),
            );
        });
    }

    #[test]
    fn test_transfer_internal_ok_fee_from_borrow() {
        new_test_ext().execute_with(|| {
            init_usdc_asset().unwrap();
            let amount: AssetQuantity = usdc.as_quantity_nominal("1");

            init_asset_balance(Usdc, account_a, Balance::from_nominal("10", USD).value);
            init_asset_balance(Usdc, account_b, Balance::from_nominal("100", USD).value);

            transfer_internal::<Test>(usdc, account_a, account_b, amount)
                .expect("transfer success");

            assert_eq!(
                TotalSupplyAssets::get(Usdc),
                Quantity::from_nominal("110", USD).value
            );
            assert_eq!(
                TotalBorrowAssets::get(Usdc),
                Quantity::from_nominal("0", USD).value
            );
            assert_eq!(
                AssetBalances::get(Usdc, account_a),
                Balance::from_nominal("9", USD).value
            );
            assert_eq!(
                AssetBalances::get(Usdc, account_b),
                Balance::from_nominal("101", USD).value
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_a).collect::<Vec<_>>(),
                vec![(Usdc, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_b).collect::<Vec<_>>(),
                vec![(Usdc, ())]
            );
            assert_eq!(
                LastIndices::get(Usdc, account_a),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Usdc, account_b),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(account_a),
                CashPrincipal::from_nominal("-0.01")
            );
            assert_eq!(
                CashPrincipals::get(account_b),
                CashPrincipal::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(miner),
                CashPrincipal::from_nominal("0.01")
            );
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("0.01")
            );
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                CashPrincipalAmount::from_nominal("0")
            );
        });
    }

    #[test]
    fn test_transfer_internal_ok_fee_from_cash() {
        new_test_ext().execute_with(|| {
            init_usdc_asset().unwrap();
            let amount: AssetQuantity = usdc.as_quantity_nominal("1");

            init_asset_balance(Usdc, account_b, Balance::from_nominal("100", USD).value);
            init_cash(account_a, CashPrincipal::from_nominal("100"));

            transfer_internal::<Test>(usdc, account_a, account_b, amount)
                .expect("transfer success");

            assert_eq!(
                TotalSupplyAssets::get(Usdc),
                Quantity::from_nominal("101", USD).value
            );
            assert_eq!(
                TotalBorrowAssets::get(Usdc),
                Quantity::from_nominal("1", USD).value
            );
            assert_eq!(
                AssetBalances::get(Usdc, account_a),
                Balance::from_nominal("-1", USD).value
            );
            assert_eq!(
                AssetBalances::get(Usdc, account_b),
                Balance::from_nominal("101", USD).value
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_a).collect::<Vec<_>>(),
                vec![(Usdc, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_b).collect::<Vec<_>>(),
                vec![(Usdc, ())]
            );
            assert_eq!(
                LastIndices::get(Usdc, account_a),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Usdc, account_b),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(account_a),
                CashPrincipal::from_nominal("99.99")
            );
            assert_eq!(
                CashPrincipals::get(account_b),
                CashPrincipal::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(miner),
                CashPrincipal::from_nominal("0.01")
            );
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("100")
            );
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                CashPrincipalAmount::from_nominal("100")
            );
        });
    }

    #[test]
    fn test_transfer_cash_principal_internal_below_min() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("0.1");

            assert_eq!(
                transfer_cash_principal_internal::<Test>(account_a, account_b, principal),
                Err(Reason::MinTxValueNotMet),
            );
        });
    }

    #[test]
    fn test_transfer_cash_principal_internal_undercollateralized() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("1");

            assert_eq!(
                transfer_cash_principal_internal::<Test>(account_a, account_b, principal),
                Err(Reason::InsufficientLiquidity),
            );
        });
    }

    #[test]
    fn test_transfer_cash_principal_internal_undercollateralized_for_fee() {
        new_test_ext().execute_with(|| {
            init_usdc_asset().unwrap();
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("1");

            init_cash(account_a, CashPrincipal::from_nominal("1"));

            assert_eq!(
                transfer_cash_principal_internal::<Test>(account_a, account_b, principal),
                Err(Reason::InsufficientLiquidity),
            );
        });
    }

    #[test]
    fn test_transfer_cash_principal_internal_ok_fee_from_borrow() {
        new_test_ext().execute_with(|| {
            init_usdc_asset().unwrap();
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("1");

            init_asset_balance(Usdc, account_a, Balance::from_nominal("10", USD).value);

            transfer_cash_principal_internal::<Test>(account_a, account_b, principal)
                .expect("transfer success");

            assert_eq!(
                TotalSupplyAssets::get(Usdc),
                Quantity::from_nominal("10", USD).value
            );
            assert_eq!(
                TotalBorrowAssets::get(Usdc),
                Quantity::from_nominal("0", USD).value
            );
            assert_eq!(
                AssetBalances::get(Usdc, account_a),
                Balance::from_nominal("10", USD).value
            );
            assert_eq!(
                AssetBalances::get(Usdc, account_b),
                Balance::from_nominal("0", USD).value
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_a).collect::<Vec<_>>(),
                vec![(Usdc, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_b).collect::<Vec<_>>(),
                vec![]
            );
            assert_eq!(
                CashPrincipals::get(account_a),
                CashPrincipal::from_nominal("-1.01")
            );
            assert_eq!(
                CashPrincipals::get(account_b),
                CashPrincipal::from_nominal("1")
            );
            assert_eq!(
                CashPrincipals::get(miner),
                CashPrincipal::from_nominal("0.01")
            );
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("1.01")
            );
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                CashPrincipalAmount::from_nominal("0")
            );
        });
    }

    #[test]
    fn test_transfer_cash_principal_internal_ok_fee_from_cash() {
        new_test_ext().execute_with(|| {
            let principal: CashPrincipalAmount = CashPrincipalAmount::from_nominal("1");

            init_cash(account_a, CashPrincipal::from_nominal("2"));
            init_cash(account_b, CashPrincipal::from_nominal("100"));

            transfer_cash_principal_internal::<Test>(account_a, account_b, principal)
                .expect("transfer success");

            assert_eq!(
                CashPrincipals::get(account_a),
                CashPrincipal::from_nominal("0.99")
            );
            assert_eq!(
                CashPrincipals::get(account_b),
                CashPrincipal::from_nominal("101")
            );
            assert_eq!(
                CashPrincipals::get(miner),
                CashPrincipal::from_nominal("0.01")
            );
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("102")
            );
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                CashPrincipalAmount::from_nominal("102")
            );
        });
    }
}
