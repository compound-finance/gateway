use frame_support::storage::{
    IterableStorageDoubleMap, StorageDoubleMap, StorageMap, StorageValue,
};
use our_std::collections::btree_map::BTreeMap;
use our_std::RuntimeDebug;

use crate::{
    chains::ChainAccount,
    core::get_some_miner,
    internal::balance_helpers::*,
    portfolio::Portfolio,
    reason::Reason,
    types::{
        AssetBalance, AssetIndex, AssetInfo, Balance, CashPrincipal, CashPrincipalAmount, Quantity,
    },
    AssetAmount, AssetBalances, AssetsWithNonZeroBalance, BorrowIndices, CashPrincipals,
    ChainAsset, Config, GlobalCashIndex, LastIndices, SupplyIndices, SupportedAssets,
    TotalBorrowAssets, TotalCashPrincipal, TotalSupplyAssets,
};

trait Apply {
    fn apply<T: Config>(self: &Self, state: &State) -> Result<State, Reason>;
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
        }
    }

    pub fn build_portfolio<T: Config>(
        self: Self,
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

    pub fn get_total_supply_asset<T: Config>(self: &mut Self, asset_info: AssetInfo) -> Quantity {
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

    pub fn get_total_borrow_asset<T: Config>(self: &mut Self, asset_info: AssetInfo) -> Quantity {
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

    pub fn commit<T: Config>(self: Self) {
        self.total_supply_asset
            .into_iter()
            .for_each(|(chain_asset, asset_amount)| {
                TotalSupplyAssets::insert(chain_asset, asset_amount);
            });
        self.total_borrow_asset
            .into_iter()
            .for_each(|(chain_asset, asset_amount)| {
                TotalBorrowAssets::insert(chain_asset, asset_amount);
            });
        self.asset_balances
            .into_iter()
            .for_each(|((chain_asset, account), balance)| {
                AssetBalances::insert(chain_asset, account, balance);
            });
        self.assets_with_non_zero_balance.into_iter().for_each(
            |((chain_asset, account), is_non_zero)| {
                if is_non_zero {
                    AssetsWithNonZeroBalance::insert(account, chain_asset, ());
                } else {
                    AssetsWithNonZeroBalance::remove(account, chain_asset);
                }
            },
        );
        self.last_indices
            .into_iter()
            .for_each(|((chain_asset, account), last_index)| {
                LastIndices::insert(chain_asset, account, last_index);
            });
        self.cash_principals
            .into_iter()
            .for_each(|(account, cash_principal)| {
                CashPrincipals::insert(account, cash_principal);
            });
    }
}

// TODO: require!(sender != recipient, Reason::SelfTransfer);
fn prepare_transfer_asset<T: Config>(
    st_pre: &State,
    sender: ChainAccount,
    recipient: ChainAccount,
    asset: ChainAsset,
    quantity: Quantity,
) -> Result<State, Reason> {
    let mut st: State = st_pre.clone();

    if (sender == recipient) {
        Err(Reason::SelfTransfer)?
    }

    let asset_info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    let supply_index = SupplyIndices::get(asset);
    let borrow_index = BorrowIndices::get(asset);
    let total_supply_pre = st.get_total_supply_asset::<T>(asset_info);
    let total_borrow_pre = st.get_total_borrow_asset::<T>(asset_info);
    let sender_balance_pre = st.get_asset_balance::<T>(asset_info, sender);
    let recipient_balance_pre = st.get_asset_balance::<T>(asset_info, recipient);
    let sender_last_index_pre = st.get_last_index::<T>(asset_info, sender);
    let recipient_last_index_pre = st.get_last_index::<T>(asset_info, recipient);
    let sender_cash_principal_pre = st.get_cash_principal::<T>(sender);
    let recipient_cash_principal_pre = st.get_cash_principal::<T>(recipient);

    let (sender_withdraw_amount, sender_borrow_amount) =
        withdraw_and_borrow_amount(sender_balance_pre.value, quantity)?;
    let (recipient_repay_amount, recipient_supply_amount) =
        repay_and_supply_amount(recipient_balance_pre.value, quantity)?;

    let total_supply_new = total_supply_pre
        .add(recipient_supply_amount)?
        .sub(sender_withdraw_amount)
        .map_err(|_| Reason::InsufficientTotalFunds)?;

    let total_borrow_new = total_borrow_pre
        .add(sender_borrow_amount)?
        .sub(recipient_repay_amount)?;

    let sender_balance_post = sender_balance_pre.sub_quantity(quantity)?;
    let recipient_balance_post = recipient_balance_pre.add_quantity(quantity)?;

    let (sender_cash_principal_post, sender_last_index_post) = effect_of_asset_interest_internal(
        sender_balance_pre,
        sender_balance_post,
        sender_cash_principal_pre,
        sender_last_index_pre,
        supply_index,
        borrow_index,
    )?;

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
    st.set_asset_balance::<T>(asset_info, sender, sender_balance_post);
    st.set_asset_balance::<T>(asset_info, recipient, recipient_balance_post);
    st.set_last_index::<T>(asset_info, sender, sender_last_index_post);
    st.set_last_index::<T>(asset_info, recipient, recipient_last_index_post);
    st.set_cash_principal::<T>(sender, sender_cash_principal_post);
    st.set_cash_principal::<T>(recipient, recipient_cash_principal_post);

    Ok(st)
}

fn prepare_transfer_cash<T: Config>(
    st_pre: &State,
    sender: ChainAccount,
    recipient: ChainAccount,
    principal: CashPrincipalAmount,
    fee_principal: CashPrincipalAmount,
) -> Result<State, Reason> {
    let mut st: State = st_pre.clone();
    let miner = get_some_miner::<T>();

    let principal_with_fee = principal.add(fee_principal)?;

    let sender_cash_pre = st.get_cash_principal::<T>(sender);
    let recipient_cash_pre = st.get_cash_principal::<T>(recipient);
    let miner_cash_pre = st.get_cash_principal::<T>(miner);

    let (_sender_withdraw_principal, sender_borrow_principal) =
        withdraw_and_borrow_principal(sender_cash_pre, principal_with_fee)?;
    let (recipient_repay_principal, _recipient_supply_principal) =
        repay_and_supply_principal(recipient_cash_pre, principal)?;
    let (miner_repay_principal, _miner_supply_principal) =
        repay_and_supply_principal(miner_cash_pre, fee_principal)?;

    let miner_cash_post = miner_cash_pre.add_amount(fee_principal)?;
    let sender_cash_post = sender_cash_pre.sub_amount(principal_with_fee)?;
    let recipient_cash_post = recipient_cash_pre.add_amount(principal)?;

    let total_cash_post = st
        .get_total_cash_principal::<T>()
        .add(sender_borrow_principal)?
        .sub(recipient_repay_principal)?
        .sub(miner_repay_principal)?;

    st.set_cash_principal::<T>(miner, miner_cash_post);
    st.set_cash_principal::<T>(sender, sender_cash_post);
    st.set_cash_principal::<T>(recipient, recipient_cash_post);
    st.set_total_cash_principal::<T>(total_cash_post);

    Ok(st)
}

#[derive(Clone, Eq, PartialEq, RuntimeDebug)]
pub enum Effect {
    Init,
    TransferAsset {
        sender: ChainAccount,
        recipient: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    },
}

impl Apply for Effect {
    fn apply<T: Config>(self: &Self, state: &State) -> Result<State, Reason> {
        match self {
            Effect::Init => Ok(state.clone()),
            Effect::TransferAsset {
                sender,
                recipient,
                asset,
                quantity,
            } => prepare_transfer_asset::<T>(state, *sender, *recipient, *asset, *quantity),
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

    fn state(self: &Self) -> &State {
        &self.effects.last().expect("expected last state").1
    }

    pub fn transfer_asset<T: Config>(
        self: &mut Self,
        sender: ChainAccount,
        recipient: ChainAccount,
        asset: ChainAsset,
        quantity: Quantity,
    ) -> Result<&mut Self, Reason> {
        let effect = Effect::TransferAsset {
            sender,
            recipient,
            asset,
            quantity,
        };
        let state_old = self.state();
        let state_new = effect.apply::<T>(state_old)?;
        self.effects.push((effect, state_new));
        Ok(self)
    }

    pub fn check_liquidity<T: Config>(
        self: &mut Self,
        account: ChainAccount,
    ) -> Result<&mut Self, Reason> {
        // TODO: Consider removing all the state cloning
        let liquidity = self
            .state()
            .clone()
            .build_portfolio::<T>(account)?
            .get_liquidity::<T>()?;
        if liquidity.value < 0 {
            Err(Reason::InsufficientLiquidity)?
        } else {
            Ok(self)
        }
    }

    pub fn commit<T: Config>(self: &mut Self) {
        // TODO: Consider removing all the state cloning
        self.state().clone().commit::<T>();
    }
}

// TODO: Maybe share a purified version with core.rs?
/// Return CASH Principal post asset interest, and updated asset index
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::*,
        tests::{assert_ok, assets::*, common::*, mock::*},
        types::*,
    };

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

            let state = Effectful::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, quantity)
                .expect("transfer_asset failed")
                .state()
                .clone();

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

            let state = Effectful::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .state()
                .clone();

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
    fn test_check_liquidity() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(init_wbtc_asset());

            CashPrincipals::insert(account_a, CashPrincipal(100000000000)); // 100,000 CASH
            AssetsWithNonZeroBalance::insert(account_a, Wbtc, ());

            let eth_quantity = eth.as_quantity_nominal("1");
            let wbtc_quantity = wbtc.as_quantity_nominal("0.02");

            let mut effect = Effectful::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .clone();

            assert_ok!(effect.check_liquidity::<Test>(account_a));
            assert_eq!(
                effect.check_liquidity::<Test>(account_b),
                Err(Reason::InsufficientLiquidity)
            );
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

            let state = Effectful::new()
                .transfer_asset::<Test>(account_a, account_b, Eth, eth_quantity)
                .expect("transfer_asset(eth) failed")
                .transfer_asset::<Test>(account_b, account_a, Wbtc, wbtc_quantity)
                .expect("transfer_asset(wbtc) failed")
                .state()
                .clone();

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

            Effectful::new()
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

            Effectful::new()
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
        })
    }
}
