use codec::{Decode, Encode};
use frame_support::storage::{IterableStorageDoubleMap, StorageDoubleMap, StorageMap};
use our_std::RuntimeDebug;

use crate::{
    chains::ChainAccount,
    core::{get_asset, get_cash_balance_with_asset_interest, get_price},
    reason::Reason,
    symbol::CASH,
    types::{AssetInfo, Balance, Quantity},
    AssetAmount, AssetBalances, AssetsWithNonZeroBalance, ChainAsset, Config, TotalBorrowAssets,
    TotalSupplyAssets,
};

use types_derive::Types;

trait Apply {
    fn apply<T: Config>(self: &Self, state: &State) -> Result<State, Reason>;
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct State {
    total_supply_asset_: Vec<(ChainAsset, AssetAmount)>,
    total_borrow_asset_: Vec<(ChainAsset, AssetAmount)>,
}

impl State {
    pub fn new() -> Self {
        State {
            total_supply_asset_: vec![],
            total_borrow_asset_: vec![],
        }
    }

    pub fn get_total_supply_asset<T: Config>(self: &mut Self, asset: ChainAsset) -> AssetAmount {
        find_or_fetch(
            &mut self.total_supply_asset_,
            |el| el.0 == asset,
            || (asset, TotalSupplyAssets::get(asset)),
            |el| el.1,
        )
    }

    pub fn set_total_supply_asset<T: Config>(
        self: &mut Self,
        asset: ChainAsset,
        amount: AssetAmount,
    ) {
        set_or_push(
            &mut self.total_supply_asset_,
            |el| el.0 == asset,
            (asset, amount),
        )
    }

    pub fn get_total_borrow_asset<T: Config>(self: &mut Self, asset: ChainAsset) -> AssetAmount {
        find_or_fetch(
            &mut self.total_borrow_asset_,
            |el| el.0 == asset,
            || (asset, TotalBorrowAssets::get(asset)),
            |el| el.1,
        )
    }

    pub fn set_total_borrow_asset<T: Config>(
        self: &mut Self,
        asset: ChainAsset,
        amount: AssetAmount,
    ) {
        set_or_push(
            &mut self.total_borrow_asset_,
            |el| el.0 == asset,
            (asset, amount),
        )
    }
}

fn prepare_transfer_asset<T: Config>(
    st_pre: &State,
    from: ChainAccount,
    to: ChainAccount,
    asset: ChainAsset,
    quantity: Quantity,
) -> Result<State, Reason> {
    let mut st: State = st_pre.clone();

    // TODO: Get some things
    let total_supply_pre = st.get_total_supply_asset::<T>(asset);
    let total_borrow_pre = st.get_total_borrow_asset::<T>(asset);
    let from_asset_balance = AssetBalances::get(asset, from);
    let to_asset_balance = AssetBalances::get(asset, to);

    let (recipient_repay_amount, recipient_supply_amount) = repay_and_supply_amount(asset, amount)?;
    let (sender_withdraw_amount, sender_borrow_amount) = withdraw_and_borrow_amount(asset, amount)?;

    // let total_supply_new = total_supply_pre
    //     .add_amount(recipient_supply_amount)?
    //     .sub_amount(sender_withdraw_amount)
    //     .ok_or(Reason::InsufficientTotalFunds);

    // let total_borrow_new = total_borrow_pre
    //     .add_amount(sender_borrow_amount)?
    //     .sub_amount(sender_withdraw_amount)?; // Neither should happen under normal conditions

    st.set_total_supply_asset::<T>(asset, total_supply_pre);
    st.set_total_borrow_asset::<T>(asset, total_borrow_pre);

    Ok(st)
}

// fn exec_transfer_asset(from: ChainAccount, to: ChainAccount, asset_amount: ChainAssetAmount) {}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
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
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
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

fn find_or_fetch<T, V, F0, F1, F2>(vec: &mut Vec<T>, find_fn: F0, fetch_fn: F1, read_fn: F2) -> V
where
    F0: Fn(&T) -> bool,
    F1: Fn() -> T,
    F2: Fn(&T) -> V,
{
    for elem in vec.iter() {
        if find_fn(elem) {
            // Found
            return read_fn(elem);
        }
    }

    // Not found
    let fetched_value = fetch_fn();
    let res = read_fn(&fetched_value);
    vec.push(fetched_value);
    res
}

fn set_or_push<T, F>(vec: &mut Vec<T>, find_fn: F, value: T)
where
    F: Fn(&T) -> bool,
{
    for elem in vec.iter_mut() {
        if find_fn(elem) {
            *elem = value;
            return;
        }
    }
    // Not found
    vec.push(value);
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
