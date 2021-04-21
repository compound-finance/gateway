use frame_support::storage::{IterableStorageDoubleMap, StorageDoubleMap, StorageMap};
use our_std::collections::btree_map::BTreeMap;
use our_std::RuntimeDebug;

use crate::{
    chains::ChainAccount,
    core::{get_asset, get_cash_balance_with_asset_interest, get_price},
    internal::balance_helpers::*,
    reason::Reason,
    symbol::CASH,
    types::{AssetBalance, AssetInfo, Balance, Quantity},
    AssetAmount, AssetBalances, AssetsWithNonZeroBalance, ChainAsset, Config, TotalBorrowAssets,
    TotalSupplyAssets,
};

use types_derive::Types;

trait Apply {
    fn apply<T: Config>(self: &Self, state: &State) -> Result<State, Reason>;
}

#[derive(Clone, Eq, PartialEq, RuntimeDebug)]
pub struct State {
    total_supply_asset_: BTreeMap<ChainAsset, AssetAmount>,
    total_borrow_asset_: BTreeMap<ChainAsset, AssetAmount>,
    asset_balances_: BTreeMap<(ChainAsset, ChainAccount), AssetBalance>,
}

impl State {
    pub fn new() -> Self {
        State {
            total_supply_asset_: BTreeMap::new(),
            total_borrow_asset_: BTreeMap::new(),
            asset_balances_: BTreeMap::new(),
        }
    }

    pub fn get_total_supply_asset<T: Config>(self: &mut Self, asset: ChainAsset) -> AssetAmount {
        self.total_supply_asset_
            .get(&asset)
            .map(|x| *x)
            .unwrap_or_else(|| TotalSupplyAssets::get(asset))
    }

    pub fn set_total_supply_asset<T: Config>(
        self: &mut Self,
        asset: ChainAsset,
        amount: AssetAmount,
    ) {
        self.total_supply_asset_.insert(asset, amount);
    }

    pub fn get_total_borrow_asset<T: Config>(self: &mut Self, asset: ChainAsset) -> AssetAmount {
        self.total_borrow_asset_
            .get(&asset)
            .map(|x| *x)
            .unwrap_or_else(|| TotalBorrowAssets::get(asset))
    }

    pub fn set_total_borrow_asset<T: Config>(
        self: &mut Self,
        asset: ChainAsset,
        amount: AssetAmount,
    ) {
        self.total_borrow_asset_.insert(asset, amount);
    }

    pub fn get_asset_balance<T: Config>(
        self: &mut Self,
        asset: ChainAsset,
        account: ChainAccount,
    ) -> AssetBalance {
        self.asset_balances_
            .get(&(asset, account))
            .map(|x| *x)
            .unwrap_or_else(|| AssetBalances::get(asset, account))
    }

    pub fn set_asset_balance<T: Config>(
        self: &mut Self,
        asset: ChainAsset,
        account: ChainAccount,
        balance: AssetBalance,
    ) {
        self.asset_balances_.insert((asset, account), balance);
    }
}

fn prepare_transfer_asset<T: Config>(
    st_pre: &State,
    sender: ChainAccount,
    recipient: ChainAccount,
    asset: ChainAsset,
    quantity: Quantity,
) -> Result<State, Reason> {
    let mut st: State = st_pre.clone();

    // TODO: Get some things
    let total_supply_pre = st.get_total_supply_asset::<T>(asset);
    let total_borrow_pre = st.get_total_borrow_asset::<T>(asset);
    let sender_asset_balance = st.get_asset_balance::<T>(asset, sender);
    let recipient_asset_balance = st.get_asset_balance::<T>(asset, recipient);

    let (recipient_repay_amount, recipient_supply_amount) =
        repay_and_supply_amount(recipient_asset_balance, quantity)?;
    let (sender_withdraw_amount, sender_borrow_amount) =
        withdraw_and_borrow_amount(sender_asset_balance, quantity)?;

    let total_supply_new = total_supply_pre
        .add_amount(recipient_supply_amount)?
        .sub_amount(sender_withdraw_amount)
        .ok_or(Reason::InsufficientTotalFunds);

    let total_borrow_new = total_borrow_pre
        .add_amount(sender_borrow_amount)?
        .sub_amount(sender_repay_amount)?;

    st.set_total_supply_asset::<T>(asset, total_supply_pre);
    st.set_total_borrow_asset::<T>(asset, total_borrow_pre);

    // TODO: Set accounts data

    Ok(st)
}

// fn exec_transfer_asset(from: ChainAccount, to: ChainAccount, asset_amount: ChainAssetAmount) {}

#[derive(Clone, Eq, PartialEq, RuntimeDebug)]
pub enum Effect {
    Init,
    TransferAsset {
        from: ChainAccount,
        to: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    },
}

impl Apply for Effect {
    fn apply<T: Config>(self: &Self, state: &State) -> Result<State, Reason> {
        match self {
            Effect::Init => Ok(state.clone()),
            Effect::TransferAsset {
                from,
                to,
                asset,
                quantity,
            } => prepare_transfer_asset::<T>(state, *from, *to, *asset, *quantity),
        }
    }
}

/// Type for representing a set of positions for an account.
#[derive(Clone, Eq, PartialEq, RuntimeDebug)]
pub struct Effectful {
    pub effects: Vec<(Effect, State)>,
}

impl Effectful {
    pub fn new() -> Self {
        Effectful {
            effects: vec![(Effect::Init, State::new())],
        }
    }

    fn current_state(self: &Self) -> &State {
        &self.effects.last().expect("expected last state").1
    }

    pub fn transfer_asset<T: Config>(
        self: &mut Self,
        from: ChainAccount,
        to: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    ) -> Result<&mut Self, Reason> {
        let effect = Effect::TransferAsset {
            from,
            to,
            asset,
            quantity,
        };
        let state_old = self.current_state();
        let state_new = effect.apply::<T>(state_old)?;
        self.effects.push((effect, state_new));
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::*,
        tests::{assets::*, common::*, mock::*},
        types::*,
    };
    use pallet_oracle::types::Price;

    #[allow(non_upper_case_globals)]
    const account_a: ChainAccount = ChainAccount::Eth([1u8; 20]);
    #[allow(non_upper_case_globals)]
    const account_b: ChainAccount = ChainAccount::Eth([2u8; 20]);

    #[test]
    fn test_transfer_asset_success() {
        new_test_ext().execute_with(|| {
            let state = Effectful::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth.as_quantity_nominal("1"))
                .expect("transfer_asset failed")
                .current_state()
                .clone();

            println!("State={:?}", state);
            assert_eq!(state, State::new()); // Should be false
        })
    }
}
