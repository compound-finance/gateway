use frame_support::{
    storage::{IterableStorageDoubleMap, StorageDoubleMap, StorageMap, StorageValue},
    traits::StoredMap,
};
use our_std::collections::btree_map::BTreeMap;
use our_std::RuntimeDebug;
use sp_core::crypto::AccountId32;

use crate::{
    chains::{ChainAccount, ChainId},
    internal::balance_helpers::*,
    params::MIN_PRINCIPAL_GATE,
    portfolio::Portfolio,
    reason::Reason,
    types::{
        AssetBalance, AssetIndex, AssetInfo, Balance, CashPrincipal, CashPrincipalAmount, Quantity,
    },
    AssetAmount, AssetBalances, AssetsWithNonZeroBalance, BorrowIndices, CashPrincipals,
    ChainAsset, ChainCashPrincipals, Config, GlobalCashIndex, LastIndices, SupplyIndices,
    SupportedAssets, TotalBorrowAssets, TotalCashPrincipal, TotalSupplyAssets,
};

trait Apply {
    fn apply<T: Config>(self: Self, state: State) -> Result<State, Reason>;
}

#[derive(Clone, Eq, PartialEq, RuntimeDebug)]
pub struct State {
    total_supply_asset: BTreeMap<ChainAsset, AssetAmount>,
    total_borrow_asset: BTreeMap<ChainAsset, AssetAmount>,
    asset_balances: BTreeMap<(ChainAsset, ChainAccount), AssetBalance>,
    assets_with_non_zero_balance: BTreeMap<(ChainAsset, ChainAccount), bool>,
    last_indices: BTreeMap<(ChainAsset, ChainAccount), AssetIndex>,
    cash_principals: BTreeMap<ChainAccount, CashPrincipal>,
    total_cash_principal: Option<CashPrincipalAmount>,
    chain_cash_principals: BTreeMap<ChainId, CashPrincipalAmount>,
}

impl State {
    pub fn new() -> Self {
        State {
            total_supply_asset: BTreeMap::new(),
            total_borrow_asset: BTreeMap::new(),
            asset_balances: BTreeMap::new(),
            assets_with_non_zero_balance: BTreeMap::new(),
            last_indices: BTreeMap::new(),
            cash_principals: BTreeMap::new(),
            total_cash_principal: None,
            chain_cash_principals: BTreeMap::new(),
        }
    }

    pub fn build_portfolio<T: Config>(
        self: &Self,
        account: ChainAccount,
    ) -> Result<Portfolio, Reason> {
        let mut principal = self.get_cash_principal::<T>(account);
        let global_cash_index = GlobalCashIndex::get();

        let mut positions = Vec::new();
        for asset in self.get_assets_with_non_zero_balance::<T>(account) {
            let asset_info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
            let supply_index = SupplyIndices::get(asset);
            let borrow_index = BorrowIndices::get(asset);
            let balance = self.get_asset_balance::<T>(asset_info, account);
            let last_index = self.get_last_index::<T>(asset_info, account);

            (principal, _) = effect_of_asset_interest_internal(
                balance,
                balance,
                principal,
                last_index,
                supply_index,
                borrow_index,
            )?;
            positions.push((asset_info, balance));
        }

        let cash = global_cash_index.cash_balance(principal)?;

        Ok(Portfolio { cash, positions })
    }

    pub fn get_total_supply_asset<T: Config>(self: &Self, asset_info: AssetInfo) -> Quantity {
        asset_info.as_quantity(
            self.total_supply_asset
                .get(&asset_info.asset)
                .map(|x| *x)
                .unwrap_or_else(|| TotalSupplyAssets::get(asset_info.asset)),
        )
    }

    pub fn set_total_supply_asset<T: Config>(
        self: &mut Self,
        asset_info: AssetInfo,
        quantity: Quantity,
    ) {
        self.total_supply_asset
            .insert(asset_info.asset, quantity.value);
    }

    pub fn get_total_borrow_asset<T: Config>(self: &Self, asset_info: AssetInfo) -> Quantity {
        asset_info.as_quantity(
            self.total_borrow_asset
                .get(&asset_info.asset)
                .map(|x| *x)
                .unwrap_or_else(|| TotalBorrowAssets::get(asset_info.asset)),
        )
    }

    pub fn set_total_borrow_asset<T: Config>(
        self: &mut Self,
        asset_info: AssetInfo,
        quantity: Quantity,
    ) {
        self.total_borrow_asset
            .insert(asset_info.asset, quantity.value);
    }

    pub fn get_asset_balance<T: Config>(
        self: &Self,
        asset_info: AssetInfo,
        account: ChainAccount,
    ) -> Balance {
        asset_info.as_balance(
            self.asset_balances
                .get(&(asset_info.asset, account))
                .map(|x| *x)
                .unwrap_or_else(|| AssetBalances::get(asset_info.asset, account)),
        )
    }

    pub fn set_asset_balance<T: Config>(
        self: &mut Self,
        asset_info: AssetInfo,
        account: ChainAccount,
        balance: Balance,
    ) {
        self.assets_with_non_zero_balance
            .insert((asset_info.asset, account), balance.value != 0);
        self.asset_balances
            .insert((asset_info.asset, account), balance.value);
    }

    // Combines ground truth and modified state to return current assets with non-zero balance
    fn get_assets_with_non_zero_balance<T: Config>(
        self: &Self,
        account: ChainAccount,
    ) -> Vec<ChainAsset> {
        let mut assets = self.assets_with_non_zero_balance.clone();
        for (asset, _) in AssetsWithNonZeroBalance::iter_prefix(account) {
            if !assets.contains_key(&(asset, account)) {
                assets.insert((asset, account), true);
            }
        }

        assets
            .iter()
            .filter_map(|((asset, account_el), is_non_zero)| {
                if account == *account_el && *is_non_zero {
                    Some(*asset)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn get_last_index<T: Config>(
        self: &Self,
        asset_info: AssetInfo,
        account: ChainAccount,
    ) -> AssetIndex {
        self.last_indices
            .get(&(asset_info.asset, account))
            .map(|x| *x)
            .unwrap_or_else(|| LastIndices::get(asset_info.asset, account))
    }

    pub fn set_last_index<T: Config>(
        self: &mut Self,
        asset_info: AssetInfo,
        account: ChainAccount,
        last_index: AssetIndex,
    ) {
        self.last_indices
            .insert((asset_info.asset, account), last_index);
    }

    pub fn get_cash_principal<T: Config>(self: &Self, account: ChainAccount) -> CashPrincipal {
        self.cash_principals
            .get(&account)
            .map(|x| *x)
            .unwrap_or_else(|| CashPrincipals::get(account))
    }

    pub fn set_cash_principal<T: Config>(
        self: &mut Self,
        account: ChainAccount,
        cash_principal: CashPrincipal,
    ) {
        self.cash_principals.insert(account, cash_principal);
    }

    pub fn get_total_cash_principal<T: Config>(self: &Self) -> CashPrincipalAmount {
        self.total_cash_principal
            .unwrap_or_else(|| TotalCashPrincipal::get())
    }

    pub fn set_total_cash_principal<T: Config>(
        self: &mut Self,
        total_cash_principal: CashPrincipalAmount,
    ) {
        self.total_cash_principal = Some(total_cash_principal);
    }

    pub fn get_chain_cash_principal<T: Config>(
        self: &Self,
        chain_id: ChainId,
    ) -> CashPrincipalAmount {
        self.chain_cash_principals
            .get(&chain_id)
            .map(|x| *x)
            .unwrap_or_else(|| ChainCashPrincipals::get(chain_id))
    }

    pub fn set_chain_cash_principal<T: Config>(
        self: &mut Self,
        chain_id: ChainId,
        chain_cash_principal: CashPrincipalAmount,
    ) {
        self.chain_cash_principals
            .insert(chain_id, chain_cash_principal);
    }

    pub fn commit<T: Config>(self: &Self) {
        self.total_supply_asset
            .iter()
            .for_each(|(chain_asset, asset_amount)| {
                TotalSupplyAssets::insert(chain_asset, asset_amount);
            });
        self.total_borrow_asset
            .iter()
            .for_each(|(chain_asset, asset_amount)| {
                TotalBorrowAssets::insert(chain_asset, asset_amount);
            });
        self.asset_balances
            .iter()
            .for_each(|((chain_asset, account), balance)| {
                AssetBalances::insert(chain_asset, account, balance);
            });
        self.assets_with_non_zero_balance.iter().for_each(
            |((chain_asset, account), is_non_zero)| {
                if *is_non_zero {
                    AssetsWithNonZeroBalance::insert(account, chain_asset, ());
                } else {
                    AssetsWithNonZeroBalance::remove(account, chain_asset);
                }
            },
        );
        self.last_indices
            .iter()
            .for_each(|((chain_asset, account), last_index)| {
                LastIndices::insert(chain_asset, account, last_index);
            });
        self.cash_principals
            .iter()
            .for_each(|(account, cash_principal)| {
                CashPrincipals::insert(account, cash_principal);

                // Existential balance for Gateway accounts...
                match account {
                    ChainAccount::Gate(gate_address) => {
                        // Note: Technically we could just inc_provider/dec_provider
                        //  however we only want to do so when the state actually changes.
                        //  For now we just inefficiently write to the StoredMap each time principal changes.
                        // Also note: Technically these StoredMap calls can fail (though probably provably safe),
                        //  which would presumably trigger the underlying panic this is meant to avoid.
                        if cash_principal >= &MIN_PRINCIPAL_GATE {
                            _ = T::AccountStore::insert(&AccountId32::new(*gate_address), ());
                        } else {
                            _ = T::AccountStore::remove(&AccountId32::new(*gate_address));
                        }
                    }

                    _ => {}
                }
            });
        if let Some(total_cash_principal_new) = self.total_cash_principal {
            TotalCashPrincipal::put(total_cash_principal_new);
        }
        self.chain_cash_principals
            .iter()
            .for_each(|(chain_id, chain_cash_principal)| {
                ChainCashPrincipals::insert(chain_id, chain_cash_principal);
            });
    }
}

fn prepare_augment_asset<T: Config>(
    mut st: State,
    recipient: ChainAccount,
    asset: ChainAsset,
    quantity: Quantity,
) -> Result<State, Reason> {
    let asset_info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    let supply_index = SupplyIndices::get(asset);
    let borrow_index = BorrowIndices::get(asset);
    let total_supply_pre = st.get_total_supply_asset::<T>(asset_info);
    let total_borrow_pre = st.get_total_borrow_asset::<T>(asset_info);
    let recipient_balance_pre = st.get_asset_balance::<T>(asset_info, recipient);
    let recipient_last_index_pre = st.get_last_index::<T>(asset_info, recipient);
    let recipient_cash_principal_pre = st.get_cash_principal::<T>(recipient);

    let (recipient_repay_amount, recipient_supply_amount) =
        repay_and_supply_amount(recipient_balance_pre.value, quantity)?;

    let total_supply_new = total_supply_pre.add(recipient_supply_amount)?;

    let total_borrow_new = total_borrow_pre
        .sub(recipient_repay_amount)
        .map_err(|_| Reason::TotalBorrowUnderflow)?;

    let recipient_balance_post = recipient_balance_pre.add_quantity(quantity)?;

    let (recipient_cash_principal_post, recipient_last_index_post) =
        effect_of_asset_interest_internal(
            recipient_balance_pre,
            recipient_balance_post,
            recipient_cash_principal_pre,
            recipient_last_index_pre,
            supply_index,
            borrow_index,
        )?;

    st.set_total_supply_asset::<T>(asset_info, total_supply_new);
    st.set_total_borrow_asset::<T>(asset_info, total_borrow_new);
    st.set_asset_balance::<T>(asset_info, recipient, recipient_balance_post);
    st.set_last_index::<T>(asset_info, recipient, recipient_last_index_post);
    st.set_cash_principal::<T>(recipient, recipient_cash_principal_post);

    Ok(st)
}

fn prepare_reduce_asset<T: Config>(
    mut st: State,
    sender: ChainAccount,
    asset: ChainAsset,
    quantity: Quantity,
) -> Result<State, Reason> {
    let asset_info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    let supply_index = SupplyIndices::get(asset);
    let borrow_index = BorrowIndices::get(asset);
    let total_supply_pre = st.get_total_supply_asset::<T>(asset_info);
    let total_borrow_pre = st.get_total_borrow_asset::<T>(asset_info);
    let sender_balance_pre = st.get_asset_balance::<T>(asset_info, sender);
    let sender_last_index_pre = st.get_last_index::<T>(asset_info, sender);
    let sender_cash_principal_pre = st.get_cash_principal::<T>(sender);

    let (sender_withdraw_amount, sender_borrow_amount) =
        withdraw_and_borrow_amount(sender_balance_pre.value, quantity)?;

    let total_supply_new = total_supply_pre
        .sub(sender_withdraw_amount)
        .map_err(|_| Reason::InsufficientTotalFunds)?;

    let total_borrow_new = total_borrow_pre.add(sender_borrow_amount)?;

    let sender_balance_post = sender_balance_pre.sub_quantity(quantity)?;

    let (sender_cash_principal_post, sender_last_index_post) = effect_of_asset_interest_internal(
        sender_balance_pre,
        sender_balance_post,
        sender_cash_principal_pre,
        sender_last_index_pre,
        supply_index,
        borrow_index,
    )?;

    st.set_total_supply_asset::<T>(asset_info, total_supply_new);
    st.set_total_borrow_asset::<T>(asset_info, total_borrow_new);
    st.set_asset_balance::<T>(asset_info, sender, sender_balance_post);
    st.set_last_index::<T>(asset_info, sender, sender_last_index_post);
    st.set_cash_principal::<T>(sender, sender_cash_principal_post);

    Ok(st)
}

fn prepare_augment_cash<T: Config>(
    mut st: State,
    recipient: ChainAccount,
    principal: CashPrincipalAmount,
    from_external: bool,
) -> Result<State, Reason> {
    let recipient_cash_pre = st.get_cash_principal::<T>(recipient);

    let (recipient_repay_principal, _recipient_supply_principal) =
        repay_and_supply_principal(recipient_cash_pre, principal)?;

    let recipient_cash_post = recipient_cash_pre.add_amount(principal)?;
    let total_cash_post = st
        .get_total_cash_principal::<T>()
        .sub(recipient_repay_principal)
        .map_err(|_| Reason::InsufficientChainCash)?;

    st.set_cash_principal::<T>(recipient, recipient_cash_post);
    st.set_total_cash_principal::<T>(total_cash_post);

    if from_external {
        let chain_id = recipient.chain_id();
        let chain_cash_principal_post = st
            .get_chain_cash_principal::<T>(chain_id)
            .sub(principal)
            .map_err(|_| Reason::NegativeChainCash)?;
        st.set_chain_cash_principal::<T>(chain_id, chain_cash_principal_post);
    }

    Ok(st)
}

fn prepare_reduce_cash<T: Config>(
    mut st: State,
    sender: ChainAccount,
    principal: CashPrincipalAmount,
    to_external: bool,
) -> Result<State, Reason> {
    let sender_cash_pre = st.get_cash_principal::<T>(sender);

    let (_sender_withdraw_principal, sender_borrow_principal) =
        withdraw_and_borrow_principal(sender_cash_pre, principal)?;

    let sender_cash_post = sender_cash_pre.sub_amount(principal)?;
    let total_cash_post = st
        .get_total_cash_principal::<T>()
        .add(sender_borrow_principal)?;

    st.set_cash_principal::<T>(sender, sender_cash_post);
    st.set_total_cash_principal::<T>(total_cash_post);

    if to_external {
        let chain_id = sender.chain_id();
        let chain_cash_principal_post =
            st.get_chain_cash_principal::<T>(chain_id).add(principal)?;
        st.set_chain_cash_principal::<T>(chain_id, chain_cash_principal_post);
    }

    Ok(st)
}

#[derive(Clone, Eq, PartialEq, RuntimeDebug)]
pub enum Effect {
    AugmentAsset {
        recipient: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    },
    ReduceAsset {
        sender: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    },
    AugmentCash {
        recipient: ChainAccount,
        principal: CashPrincipalAmount,
        from_external: bool,
    },
    ReduceCash {
        sender: ChainAccount,
        principal: CashPrincipalAmount,
        to_external: bool,
    },
}

impl Apply for Effect {
    fn apply<T: Config>(self: Self, state: State) -> Result<State, Reason> {
        match self {
            Effect::AugmentAsset {
                recipient,
                asset,
                quantity,
            } => prepare_augment_asset::<T>(state, recipient, asset, quantity),
            Effect::ReduceAsset {
                sender,
                asset,
                quantity,
            } => prepare_reduce_asset::<T>(state, sender, asset, quantity),
            Effect::AugmentCash {
                recipient,
                principal,
                from_external,
            } => prepare_augment_cash::<T>(state, recipient, principal, from_external),
            Effect::ReduceCash {
                sender,
                principal,
                to_external,
            } => prepare_reduce_cash::<T>(state, sender, principal, to_external),
        }
    }
}

/// Type for representing a set of positions for an account.
#[derive(Clone, Eq, PartialEq, RuntimeDebug)]
pub struct CashPipeline {
    pub effects: Vec<Effect>,
    pub state: State,
}

impl CashPipeline {
    pub fn new() -> Self {
        CashPipeline {
            effects: vec![],
            state: State::new(),
        }
    }

    fn apply_effect<T: Config>(mut self: Self, effect: Effect) -> Result<Self, Reason> {
        self.state = effect.clone().apply::<T>(self.state)?;
        self.effects.push(effect);
        Ok(self)
    }

    pub fn transfer_asset<T: Config>(
        self: Self,
        sender: ChainAccount,
        recipient: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    ) -> Result<Self, Reason> {
        if sender == recipient {
            Err(Reason::SelfTransfer)?
        }
        self.apply_effect::<T>(Effect::AugmentAsset {
            recipient,
            asset,
            quantity,
        })?
        .apply_effect::<T>(Effect::ReduceAsset {
            sender,
            asset,
            quantity,
        })
    }

    pub fn lock_asset<T: Config>(
        self: Self,
        recipient: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    ) -> Result<Self, Reason> {
        self.apply_effect::<T>(Effect::AugmentAsset {
            recipient,
            asset,
            quantity,
        })
    }

    pub fn extract_asset<T: Config>(
        self: Self,
        sender: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    ) -> Result<Self, Reason> {
        self.apply_effect::<T>(Effect::ReduceAsset {
            sender,
            asset,
            quantity,
        })
    }

    pub fn transfer_cash<T: Config>(
        self: Self,
        sender: ChainAccount,
        recipient: ChainAccount,
        principal: CashPrincipalAmount,
    ) -> Result<Self, Reason> {
        if sender == recipient {
            Err(Reason::SelfTransfer)?
        }
        self.apply_effect::<T>(Effect::ReduceCash {
            sender,
            principal,
            to_external: false,
        })?
        .apply_effect::<T>(Effect::AugmentCash {
            recipient,
            principal,
            from_external: false,
        })
    }

    pub fn lock_cash<T: Config>(
        self: Self,
        recipient: ChainAccount,
        principal: CashPrincipalAmount,
    ) -> Result<Self, Reason> {
        self.apply_effect::<T>(Effect::AugmentCash {
            recipient,
            principal,
            from_external: true,
        })
    }

    pub fn extract_cash<T: Config>(
        self: Self,
        sender: ChainAccount,
        principal: CashPrincipalAmount,
    ) -> Result<Self, Reason> {
        self.apply_effect::<T>(Effect::ReduceCash {
            sender,
            principal,
            to_external: true,
        })
    }

    pub fn check_collateralized<T: Config>(
        self: Self,
        account: ChainAccount,
    ) -> Result<Self, Reason> {
        let liquidity = self
            .state
            .build_portfolio::<T>(account)?
            .get_liquidity::<T>()?;
        if liquidity.value < 0 {
            Err(Reason::InsufficientLiquidity)?
        } else {
            Ok(self)
        }
    }

    pub fn check_underwater<T: Config>(self: Self, account: ChainAccount) -> Result<Self, Reason> {
        let liquidity = self
            .state
            .build_portfolio::<T>(account)?
            .get_liquidity::<T>()?;
        if liquidity.value > 0 {
            Err(Reason::SufficientLiquidity)?
        } else {
            Ok(self)
        }
    }

    pub fn check_asset_balance<T: Config, F>(
        self: Self,
        account: ChainAccount,
        asset: AssetInfo,
        check_fn: F,
    ) -> Result<Self, Reason>
    where
        F: FnOnce(Balance) -> Result<(), Reason>,
    {
        let balance = self.state.get_asset_balance::<T>(asset, account);
        check_fn(balance)?;
        Ok(self)
    }

    pub fn check_cash_principal<T: Config, F>(
        self: Self,
        account: ChainAccount,
        check_fn: F,
    ) -> Result<Self, Reason>
    where
        F: FnOnce(CashPrincipal) -> Result<(), Reason>,
    {
        let principal = self.state.get_cash_principal::<T>(account);
        check_fn(principal)?;
        Ok(self)
    }

    // TODO: Do we need this check on other functions?
    pub fn check_sufficient_total_funds<T: Config>(
        self: Self,
        asset_info: AssetInfo,
    ) -> Result<Self, Reason> {
        let total_supply_asset = self.state.get_total_supply_asset::<T>(asset_info);
        let total_borrow_asset = self.state.get_total_borrow_asset::<T>(asset_info);
        if total_borrow_asset > total_supply_asset {
            Err(Reason::InsufficientTotalFunds)?
        }
        Ok(self)
    }

    pub fn commit<T: Config>(self: Self) {
        self.state.commit::<T>();
    }
}

/// Return CASH Principal including asset interest, and a new asset index,
///  given a balance change, the previous position, and current market global indices.
fn effect_of_asset_interest_internal(
    balance_old: Balance,
    balance_new: Balance,
    cash_principal_pre: CashPrincipal,
    last_index: AssetIndex,
    supply_index: AssetIndex,
    borrow_index: AssetIndex,
) -> Result<(CashPrincipal, AssetIndex), Reason> {
    let cash_index = if balance_old.value >= 0 {
        supply_index
    } else {
        borrow_index
    };
    let cash_principal_delta = cash_index.cash_principal_since(last_index, balance_old)?;
    let cash_principal_post = cash_principal_pre.add(cash_principal_delta)?;
    let last_index_post = if balance_new.value >= 0 {
        supply_index
    } else {
        borrow_index
    };
    Ok((cash_principal_post, last_index_post))
}

pub fn load_portfolio<T: Config>(account: ChainAccount) -> Result<Portfolio, Reason> {
    CashPipeline::new().state.build_portfolio::<T>(account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::*,
        tests::{assert_ok, assets::*, common::*, mock::*},
        types::*,
    };
    use our_std::convert::TryInto;

    #[allow(non_upper_case_globals)]
    const account_a: ChainAccount = ChainAccount::Eth([1u8; 20]);
    #[allow(non_upper_case_globals)]
    const account_b: ChainAccount = ChainAccount::Eth([2u8; 20]);

    #[test]
    fn test_transfer_asset_success_state() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let quantity = eth.as_quantity_nominal("1");
            let amount = quantity.value as i128;

            let state = CashPipeline::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, quantity)
                .expect("transfer_asset failed")
                .state;

            assert_eq!(
                state,
                State {
                    total_supply_asset: vec![(Eth, quantity.value)].into_iter().collect(),
                    total_borrow_asset: vec![(Eth, quantity.value)].into_iter().collect(),
                    asset_balances: vec![((Eth, account_a), -amount), ((Eth, account_b), amount)]
                        .into_iter()
                        .collect(),
                    assets_with_non_zero_balance: vec![
                        ((Eth, account_a), true),
                        ((Eth, account_b), true)
                    ]
                    .into_iter()
                    .collect(),
                    last_indices: vec![
                        ((Eth, account_a), AssetIndex::from_nominal("0")),
                        ((Eth, account_b), AssetIndex::from_nominal("0"))
                    ]
                    .into_iter()
                    .collect(),
                    cash_principals: vec![
                        (account_a, CashPrincipal::from_nominal("0")),
                        (account_b, CashPrincipal::from_nominal("0")),
                    ]
                    .into_iter()
                    .collect(),
                    total_cash_principal: None,
                    chain_cash_principals: vec![].into_iter().collect(),
                }
            );
        })
    }

    #[test]
    fn test_lock_asset_success_state() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let quantity = eth.as_quantity_nominal("1");
            let amount = quantity.value as i128;

            let state = CashPipeline::new()
                .lock_asset::<Test>(account_a, Eth, quantity)
                .expect("lock_asset failed")
                .state;

            assert_eq!(
                state,
                State {
                    total_supply_asset: vec![(Eth, quantity.value)].into_iter().collect(),
                    total_borrow_asset: vec![(Eth, 0)].into_iter().collect(),
                    asset_balances: vec![((Eth, account_a), amount)].into_iter().collect(),
                    assets_with_non_zero_balance: vec![((Eth, account_a), true)]
                        .into_iter()
                        .collect(),
                    last_indices: vec![((Eth, account_a), AssetIndex::from_nominal("0"))]
                        .into_iter()
                        .collect(),
                    cash_principals: vec![(account_a, CashPrincipal::from_nominal("0")),]
                        .into_iter()
                        .collect(),
                    total_cash_principal: None,
                    chain_cash_principals: vec![].into_iter().collect(),
                }
            );
        })
    }

    #[test]
    fn test_extract_asset_success_state() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let quantity = eth.as_quantity_nominal("1");
            let amount = quantity.value as i128;

            let state = CashPipeline::new()
                .extract_asset::<Test>(account_a, Eth, quantity)
                .expect("extract_asset failed")
                .state;

            assert_eq!(
                state,
                State {
                    total_supply_asset: vec![(Eth, 0)].into_iter().collect(),
                    total_borrow_asset: vec![(Eth, quantity.value)].into_iter().collect(),
                    asset_balances: vec![((Eth, account_a), -amount)].into_iter().collect(),
                    assets_with_non_zero_balance: vec![((Eth, account_a), true)]
                        .into_iter()
                        .collect(),
                    last_indices: vec![((Eth, account_a), AssetIndex::from_nominal("0"))]
                        .into_iter()
                        .collect(),
                    cash_principals: vec![(account_a, CashPrincipal::from_nominal("0")),]
                        .into_iter()
                        .collect(),
                    total_cash_principal: None,
                    chain_cash_principals: vec![].into_iter().collect(),
                }
            );
        })
    }

    #[test]
    fn test_transfer_cash_success_state() {
        new_test_ext().execute_with(|| {
            let quantity = CashPrincipalAmount::from_nominal("1");

            let state = CashPipeline::new()
                .transfer_cash::<Test>(account_a, account_b, quantity)
                .expect("transfer_cash failed")
                .state;

            assert_eq!(
                state,
                State {
                    total_supply_asset: vec![].into_iter().collect(),
                    total_borrow_asset: vec![].into_iter().collect(),
                    asset_balances: vec![].into_iter().collect(),
                    assets_with_non_zero_balance: vec![].into_iter().collect(),
                    last_indices: vec![].into_iter().collect(),
                    cash_principals: vec![
                        (account_a, CashPrincipal::from_nominal("-1")),
                        (account_b, CashPrincipal::from_nominal("1")),
                    ]
                    .into_iter()
                    .collect(),
                    total_cash_principal: Some(quantity),
                    chain_cash_principals: vec![].into_iter().collect(),
                }
            );
        })
    }

    #[test]
    fn test_lock_cash_success_state() {
        new_test_ext().execute_with(|| {
            ChainCashPrincipals::insert(ChainId::Eth, CashPrincipalAmount::from_nominal("3"));
            let quantity = CashPrincipalAmount::from_nominal("1");

            let state = CashPipeline::new()
                .lock_cash::<Test>(account_a, quantity)
                .expect("lock_cash failed")
                .state;

            assert_eq!(
                state,
                State {
                    total_supply_asset: vec![].into_iter().collect(),
                    total_borrow_asset: vec![].into_iter().collect(),
                    asset_balances: vec![].into_iter().collect(),
                    assets_with_non_zero_balance: vec![].into_iter().collect(),
                    last_indices: vec![].into_iter().collect(),
                    cash_principals: vec![(account_a, CashPrincipal::from_nominal("1")),]
                        .into_iter()
                        .collect(),
                    total_cash_principal: Some(CashPrincipalAmount::from_nominal("0")),
                    chain_cash_principals: vec![(
                        ChainId::Eth,
                        CashPrincipalAmount::from_nominal("2")
                    )]
                    .into_iter()
                    .collect(),
                }
            );
        })
    }

    #[test]
    fn test_extract_cash_success_state() {
        new_test_ext().execute_with(|| {
            ChainCashPrincipals::insert(ChainId::Eth, CashPrincipalAmount::from_nominal("3"));
            let quantity = CashPrincipalAmount::from_nominal("1");

            let state = CashPipeline::new()
                .extract_cash::<Test>(account_a, quantity)
                .expect("extract_cash failed")
                .state;

            assert_eq!(
                state,
                State {
                    total_supply_asset: vec![].into_iter().collect(),
                    total_borrow_asset: vec![].into_iter().collect(),
                    asset_balances: vec![].into_iter().collect(),
                    assets_with_non_zero_balance: vec![].into_iter().collect(),
                    last_indices: vec![].into_iter().collect(),
                    cash_principals: vec![(account_a, CashPrincipal::from_nominal("-1")),]
                        .into_iter()
                        .collect(),
                    total_cash_principal: Some(CashPrincipalAmount::from_nominal("1")),
                    chain_cash_principals: vec![(
                        ChainId::Eth,
                        CashPrincipalAmount::from_nominal("4")
                    )]
                    .into_iter()
                    .collect(),
                }
            );
        })
    }

    #[test]
    fn test_build_portfolio() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(init_wbtc_asset());

            CashPrincipals::insert(account_a, CashPrincipal(10000000)); // 10 CASH
            AssetsWithNonZeroBalance::insert(account_a, Wbtc, ());

            let eth_quantity = eth.as_quantity_nominal("1");
            let eth_amount = eth_quantity.value as i128;

            let wbtc_quantity = wbtc.as_quantity_nominal("0.02");
            let wbtc_amount = wbtc_quantity.value as i128;

            let state = CashPipeline::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .state;

            let portfolio_a = state.clone().build_portfolio::<Test>(account_a);

            assert_eq!(
                portfolio_a,
                Ok(Portfolio {
                    cash: Balance {
                        value: 10000000,
                        units: CASH
                    },
                    positions: vec![
                        (
                            wbtc,
                            Balance {
                                value: wbtc_amount,
                                units: WBTC
                            }
                        ),
                        (
                            eth,
                            Balance {
                                value: -eth_amount,
                                units: ETH
                            }
                        ),
                    ]
                })
            );

            let portfolio_b = state.build_portfolio::<Test>(account_b);

            assert_eq!(
                portfolio_b,
                Ok(Portfolio {
                    cash: Balance {
                        value: 0,
                        units: CASH
                    },
                    positions: vec![
                        (
                            wbtc,
                            Balance {
                                value: -wbtc_amount,
                                units: WBTC
                            }
                        ),
                        (
                            eth,
                            Balance {
                                value: eth_amount,
                                units: ETH
                            }
                        ),
                    ]
                })
            );
        })
    }

    #[test]
    fn test_check_collateralized() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(init_wbtc_asset());

            CashPrincipals::insert(account_a, CashPrincipal(100000000000)); // 100,000 CASH
            AssetsWithNonZeroBalance::insert(account_a, Wbtc, ());

            let eth_quantity = eth.as_quantity_nominal("1");
            let wbtc_quantity = wbtc.as_quantity_nominal("0.02");

            let res = CashPipeline::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .check_collateralized::<Test>(account_a)
                .expect("account_a should be liquid")
                .check_collateralized::<Test>(account_b);

            assert_eq!(res, Err(Reason::InsufficientLiquidity));
        })
    }

    #[test]
    fn test_check_underwater() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(init_wbtc_asset());

            CashPrincipals::insert(account_a, CashPrincipal(100000000000)); // 100,000 CASH
            AssetsWithNonZeroBalance::insert(account_a, Wbtc, ());

            let eth_quantity = eth.as_quantity_nominal("1");
            let wbtc_quantity = wbtc.as_quantity_nominal("0.02");

            let res = CashPipeline::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .check_underwater::<Test>(account_b)
                .expect("account_b should be underwater")
                .check_underwater::<Test>(account_a);

            assert_eq!(res, Err(Reason::SufficientLiquidity));
        })
    }

    #[test]
    fn test_check_asset_balance() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(init_wbtc_asset());

            let eth_quantity = eth.as_quantity_nominal("1");
            let wbtc_quantity = wbtc.as_quantity_nominal("0.02");

            let res = CashPipeline::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .check_asset_balance::<Test, _>(account_a, eth, |balance| {
                    if balance == eth_quantity.negate().unwrap() {
                        Ok(())
                    } else {
                        Err(Reason::None)
                    }
                })
                .expect("account_a balance should be -eth_quantity")
                .check_asset_balance::<Test, _>(account_b, eth, |balance| {
                    if balance == eth_quantity.try_into().unwrap() {
                        Ok(())
                    } else {
                        Err(Reason::None)
                    }
                })
                .expect("account_b balance should be eth_quantity")
                .check_asset_balance::<Test, _>(account_a, wbtc, |balance| {
                    if balance == wbtc_quantity.try_into().unwrap() {
                        Ok(())
                    } else {
                        Err(Reason::None)
                    }
                })
                .expect("account_a balance should be wbtc_quantity")
                .check_asset_balance::<Test, _>(account_b, wbtc, |balance| {
                    if balance == wbtc_quantity.negate().unwrap() {
                        Ok(())
                    } else {
                        Err(Reason::None)
                    }
                })
                .expect("account_b balance should be -wbtc_quantity")
                .check_asset_balance::<Test, _>(account_a, wbtc, |_| Err(Reason::None));

            assert_eq!(res, Err(Reason::None));
        })
    }

    #[test]
    fn test_check_cash_principal() {
        new_test_ext().execute_with(|| {
            let quantity = CashPrincipalAmount::from_nominal("1");

            let res = CashPipeline::new()
                .transfer_cash::<Test>(account_a, account_b, quantity)
                .expect("transfer_cash failed")
                .check_cash_principal::<Test, _>(account_a, |principal| {
                    if principal.eq(-1000000) {
                        Ok(())
                    } else {
                        Err(Reason::None)
                    }
                })
                .expect("account_a principal should be -1 CASH")
                .check_cash_principal::<Test, _>(account_b, |principal| {
                    if principal.eq(1000000) {
                        Ok(())
                    } else {
                        Err(Reason::None)
                    }
                })
                .expect("account_b principal should be 1 CASH")
                .check_cash_principal::<Test, _>(account_a, |_| Err(Reason::None));

            assert_eq!(res, Err(Reason::None));
        })
    }

    #[test]
    fn test_transfer_two_assets_success_state() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(init_wbtc_asset());

            let eth_quantity = eth.as_quantity_nominal("1");
            let eth_amount = eth_quantity.value as i128;

            let wbtc_quantity = wbtc.as_quantity_nominal("0.1");
            let wbtc_amount = wbtc_quantity.value as i128;

            let state = CashPipeline::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .state;

            assert_eq!(
                state,
                State {
                    total_supply_asset: vec![
                        (Eth, eth_quantity.value),
                        (Wbtc, wbtc_quantity.value)
                    ]
                    .into_iter()
                    .collect(),
                    total_borrow_asset: vec![
                        (Eth, eth_quantity.value),
                        (Wbtc, wbtc_quantity.value)
                    ]
                    .into_iter()
                    .collect(),
                    asset_balances: vec![
                        ((Eth, account_a), -eth_amount),
                        ((Eth, account_b), eth_amount),
                        ((Wbtc, account_a), wbtc_amount),
                        ((Wbtc, account_b), -wbtc_amount)
                    ]
                    .into_iter()
                    .collect(),
                    assets_with_non_zero_balance: vec![
                        ((Eth, account_a), true),
                        ((Eth, account_b), true),
                        ((Wbtc, account_a), true),
                        ((Wbtc, account_b), true)
                    ]
                    .into_iter()
                    .collect(),
                    last_indices: vec![
                        ((Eth, account_a), AssetIndex::from_nominal("0")),
                        ((Eth, account_b), AssetIndex::from_nominal("0")),
                        ((Wbtc, account_a), AssetIndex::from_nominal("0")),
                        ((Wbtc, account_b), AssetIndex::from_nominal("0"))
                    ]
                    .into_iter()
                    .collect(),
                    cash_principals: vec![
                        (account_a, CashPrincipal::from_nominal("0")),
                        (account_b, CashPrincipal::from_nominal("0")),
                    ]
                    .into_iter()
                    .collect(),
                    total_cash_principal: None,
                    chain_cash_principals: vec![].into_iter().collect(),
                }
            );
        })
    }

    #[test]
    fn test_transfer_asset_success_commit() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let quantity = eth.as_quantity_nominal("1");
            let amount = quantity.value as i128;

            CashPipeline::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, quantity)
                .expect("transfer_asset failed")
                .commit::<Test>();

            assert_eq!(TotalSupplyAssets::get(Eth), quantity.value);
            assert_eq!(TotalBorrowAssets::get(Eth), quantity.value);
            assert_eq!(AssetBalances::get(Eth, account_a), -amount);
            assert_eq!(AssetBalances::get(Eth, account_b), amount);
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_a).collect::<Vec<_>>(),
                vec![(Eth, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_b).collect::<Vec<_>>(),
                vec![(Eth, ())]
            );
            assert_eq!(
                LastIndices::get(Eth, account_a),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Eth, account_b),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Wbtc, account_a),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Wbtc, account_b),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(account_a),
                CashPrincipal::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(account_b),
                CashPrincipal::from_nominal("0")
            );
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                CashPrincipalAmount::from_nominal("0")
            );
        })
    }

    #[test]
    fn test_transfer_two_assets_success_commit() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(init_wbtc_asset());

            let eth_quantity = eth.as_quantity_nominal("1");
            let eth_amount = eth_quantity.value as i128;

            let wbtc_quantity = wbtc.as_quantity_nominal("0.1");
            let wbtc_amount = wbtc_quantity.value as i128;

            CashPipeline::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .commit::<Test>();

            assert_eq!(TotalSupplyAssets::get(Eth), eth_quantity.value);
            assert_eq!(TotalBorrowAssets::get(Eth), eth_quantity.value);
            assert_eq!(TotalSupplyAssets::get(Wbtc), wbtc_quantity.value);
            assert_eq!(TotalBorrowAssets::get(Wbtc), wbtc_quantity.value);
            assert_eq!(AssetBalances::get(Eth, account_a), -eth_amount);
            assert_eq!(AssetBalances::get(Eth, account_b), eth_amount);
            assert_eq!(AssetBalances::get(Wbtc, account_a), wbtc_amount);
            assert_eq!(AssetBalances::get(Wbtc, account_b), -wbtc_amount);
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_a).collect::<Vec<_>>(),
                vec![(Eth, ()), (Wbtc, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_b).collect::<Vec<_>>(),
                vec![(Eth, ()), (Wbtc, ())]
            );
            assert_eq!(
                LastIndices::get(Eth, account_a),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Eth, account_b),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Wbtc, account_a),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                LastIndices::get(Wbtc, account_b),
                AssetIndex::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(account_a),
                CashPrincipal::from_nominal("0")
            );
            assert_eq!(
                CashPrincipals::get(account_b),
                CashPrincipal::from_nominal("0")
            );
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                CashPrincipalAmount::from_nominal("0")
            );
        })
    }

    #[test]
    fn test_commit() {
        new_test_ext().execute_with(|| {
            AssetsWithNonZeroBalance::insert(account_a, Eth, ());
            AssetsWithNonZeroBalance::insert(account_b, Eth, ());

            let state = State {
                total_supply_asset: vec![(Eth, 1000), (Wbtc, 2000)].into_iter().collect(),
                total_borrow_asset: vec![(Eth, 3000), (Wbtc, 4000)].into_iter().collect(),
                asset_balances: vec![
                    ((Eth, account_a), 5000),
                    ((Eth, account_b), -6000),
                    ((Wbtc, account_a), -7000),
                    ((Wbtc, account_b), 8000),
                ]
                .into_iter()
                .collect(),
                assets_with_non_zero_balance: vec![
                    ((Eth, account_a), true),
                    ((Eth, account_b), false),
                    ((Wbtc, account_a), false),
                    ((Wbtc, account_b), true),
                ]
                .into_iter()
                .collect(),
                last_indices: vec![
                    ((Eth, account_a), AssetIndex::from_nominal("9000")),
                    ((Eth, account_b), AssetIndex::from_nominal("10000")),
                    ((Wbtc, account_a), AssetIndex::from_nominal("11000")),
                    ((Wbtc, account_b), AssetIndex::from_nominal("12000")),
                ]
                .into_iter()
                .collect(),
                cash_principals: vec![
                    (account_a, CashPrincipal::from_nominal("13000")),
                    (account_b, CashPrincipal::from_nominal("14000")),
                ]
                .into_iter()
                .collect(),
                total_cash_principal: Some(CashPrincipalAmount::from_nominal("15000")),
                chain_cash_principals: vec![
                    (ChainId::Eth, CashPrincipalAmount::from_nominal("16000")),
                    (ChainId::Dot, CashPrincipalAmount::from_nominal("17000")),
                ]
                .into_iter()
                .collect(),
            };

            state.commit::<Test>();

            assert_eq!(TotalSupplyAssets::get(Eth), 1000);
            assert_eq!(TotalSupplyAssets::get(Wbtc), 2000);
            assert_eq!(TotalBorrowAssets::get(Eth), 3000);
            assert_eq!(TotalBorrowAssets::get(Wbtc), 4000);
            assert_eq!(AssetBalances::get(Eth, account_a), 5000);
            assert_eq!(AssetBalances::get(Eth, account_b), -6000);
            assert_eq!(AssetBalances::get(Wbtc, account_a), -7000);
            assert_eq!(AssetBalances::get(Wbtc, account_b), 8000);
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_a).collect::<Vec<_>>(),
                vec![(Eth, ())]
            );
            assert_eq!(
                AssetsWithNonZeroBalance::iter_prefix(account_b).collect::<Vec<_>>(),
                vec![(Wbtc, ())]
            );
            assert_eq!(
                LastIndices::get(Eth, account_a),
                AssetIndex::from_nominal("9000")
            );
            assert_eq!(
                LastIndices::get(Eth, account_b),
                AssetIndex::from_nominal("10000")
            );
            assert_eq!(
                LastIndices::get(Wbtc, account_a),
                AssetIndex::from_nominal("11000")
            );
            assert_eq!(
                LastIndices::get(Wbtc, account_b),
                AssetIndex::from_nominal("12000")
            );
            assert_eq!(
                CashPrincipals::get(account_a),
                CashPrincipal::from_nominal("13000")
            );
            assert_eq!(
                CashPrincipals::get(account_b),
                CashPrincipal::from_nominal("14000")
            );
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("15000")
            );
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                CashPrincipalAmount::from_nominal("16000")
            );
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Dot),
                CashPrincipalAmount::from_nominal("17000")
            );
        })
    }

    // #[test]
    // fn test_liquidate_internal_asset_repay_and_supply_amount_overflow() {
    //     new_test_ext().execute_with(|| {
    //         let amount: AssetQuantity = eth.as_quantity_nominal("1");

    //         init_eth_asset().unwrap();
    //         init_wbtc_asset().unwrap();

    //         init_asset_balance(Eth, borrower, Balance::from_nominal("3", ETH).value); // 3 * 2000 * 0.8 = 4800
    //         init_asset_balance(Wbtc, borrower, Balance::from_nominal("-1", WBTC).value); // 1 * 60000 / 0.6 = -100000
    //         init_cash(borrower, CashPrincipal::from_nominal("95000")); // 95000 + 4800 - 1000000 = -200

    //         init_cash(liquidator, CashPrincipal::from_nominal("1000000"));
    //         init_asset_balance(Eth, liquidator, i128::MIN);

    //         assert_eq!(
    //             liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
    //             Err(Reason::MathError(MathError::Overflow))
    //         );
    //     })
    // }

    // #[test]
    // fn test_liquidate_internal_collateral_asset_repay_and_supply_amount_overflow() {
    //     new_test_ext().execute_with(|| {
    //         let amount: AssetQuantity = eth.as_quantity_nominal("1");

    //         init_eth_asset().unwrap();
    //         init_wbtc_asset().unwrap();

    //         init_asset_balance(Eth, borrower, Balance::from_nominal("-3", ETH).value);

    //         init_cash(liquidator, CashPrincipal::from_nominal("1000000"));
    //         init_asset_balance(Wbtc, liquidator, i128::MIN);

    //         assert_eq!(
    //             liquidate_internal::<Test>(asset, collateral_asset, liquidator, borrower, amount),
    //             Err(Reason::MathError(MathError::Overflow))
    //         );
    //     })
    // }
}
