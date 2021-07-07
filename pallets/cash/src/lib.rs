#![allow(incomplete_features)]
#![feature(array_methods)]
#![feature(associated_type_defaults)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(const_panic)]
#![feature(destructuring_assignment)]

#[macro_use]
extern crate alloc;
extern crate ethereum_client;
extern crate trx_request;

use crate::{
    chains::{
        ChainAccount, ChainAccountSignature, ChainAsset, ChainBlock, ChainBlockEvent,
        ChainBlockEvents, ChainBlockTally, ChainBlocks, ChainHash, ChainId, ChainReorg,
        ChainReorgTally, ChainSignature, ChainSignatureList, ChainStarport,
    },
    notices::{Notice, NoticeId, NoticeState},
    portfolio::Portfolio,
    symbol::CASH,
    types::{
        AssetAmount, AssetBalance, AssetIndex, AssetInfo, Balance, Bips, CashIndex, CashPrincipal,
        CashPrincipalAmount, CodeHash, EncodedNotice, GovernanceResult, InterestRateModel,
        LiquidityFactor, Nonce, Reason, SessionIndex, Timestamp, ValidatorKeys, APR,
    },
};
use codec::{alloc::string::String, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, dispatch,
    traits::{StoredMap, UnfilteredDispatchable},
    weights::{DispatchClass, GetDispatchInfo, Pays, Weight},
    Parameter,
};
use frame_system;
use frame_system::{ensure_none, ensure_root, offchain::CreateSignedTransaction};
use num_traits::Zero;
use our_std::{
    collections::btree_map::BTreeMap, collections::btree_set::BTreeSet, convert::TryInto, debug,
    error, log, str, vec::Vec, warn, Debuggable,
};
use sp_core::crypto::AccountId32;
use sp_runtime::{
    traits::Convert,
    transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidity},
    Percent,
};

use pallet_oracle;
use pallet_session;
use pallet_timestamp;
use types_derive::type_alias;

#[macro_use]
extern crate lazy_static;

pub mod chains;
pub mod converters;
pub mod core;
pub mod events;
pub mod factor;
pub mod internal;
pub mod notices;
pub mod params;
pub mod pipeline;
pub mod portfolio;
pub mod rates;
pub mod reason;
pub mod require;
pub mod serdes;
pub mod symbol;
pub mod trx_req;
pub mod types;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(any(test, feature = "mocks"))]
pub mod mocks;

#[cfg(test)]
mod tests;

/// Type for linking sessions to validators.
#[type_alias]
pub type SubstrateId = AccountId32;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config:
    frame_system::Config
    + CreateSignedTransaction<Call<Self>>
    + pallet_timestamp::Config
    + pallet_oracle::Config
{
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;

    /// The overarching dispatch call type.
    type Call: From<Call<Self>>
        + Parameter
        + UnfilteredDispatchable<Origin = Self::Origin>
        + GetDispatchInfo;

    /// Convert implementation for Moment -> Timestamp.
    type TimeConverter: Convert<<Self as pallet_timestamp::Config>::Moment, Timestamp>;

    /// Placate substrate's `HandleLifetime` trait.
    type AccountStore: StoredMap<SubstrateId, ()>;

    /// Associated type which allows us to interact with substrate Sessions.
    type SessionInterface: self::SessionInterface<SubstrateId>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

decl_storage! {
    trait Store for Module<T: Config> as Cash {
        /// The timestamp of the previous block (or initialized to yield start defined in genesis).
        LastYieldTimestamp get(fn last_yield_timestamp) config(): Timestamp;

        /// A possible next code hash which is used to accept code provided to SetNextCodeViaHash.
        AllowedNextCodeHash get(fn allowed_next_code_hash): Option<CodeHash>;

        /// The upcoming session at which to tell the sessions pallet to rotate the validators.
        NextSessionIndex get(fn next_session_index): SessionIndex;

        /// The upcoming set of allowed validators, and their associated keys (or none).
        NextValidators get(fn next_validators): map hasher(blake2_128_concat) SubstrateId => Option<ValidatorKeys>;

        /// The current set of allowed validators, and their associated keys.
        Validators get(fn validators): map hasher(blake2_128_concat) SubstrateId => Option<ValidatorKeys>;

        /// An index to track interest earned by CASH holders and owed by CASH borrowers.
        /// Note - the implementation of Default for CashIndex returns ONE. This also provides
        /// the initial value as it is currently implemented.
        GlobalCashIndex get(fn cash_index): CashIndex;

        /// The upcoming base rate change for CASH and when, if any.
        CashYieldNext get(fn cash_yield_next): Option<(APR, Timestamp)>;

        /// The current APR on CASH held, and the base rate paid by borrowers.
        CashYield get(fn cash_yield) config(): APR;

        /// The liquidation incentive on seized collateral (e.g. 8% = 800 bips).
        GlobalLiquidationIncentive get(fn liquidation_incentive): Bips;

        /// The fraction of borrower interest that is paid to the protocol (e.g. 1/10th = 1000 bips).
        Spreads get(fn spread): map hasher(blake2_128_concat) ChainAsset => Bips;

        /// The mapping of indices to track interest owed by asset borrowers, by asset.
        BorrowIndices get(fn borrow_index): map hasher(blake2_128_concat) ChainAsset => AssetIndex;

        /// The mapping of indices to track interest earned by asset suppliers, by asset.
        SupplyIndices get(fn supply_index): map hasher(blake2_128_concat) ChainAsset => AssetIndex;

        /// The total CASH principal held per chain.
        ChainCashPrincipals get(fn chain_cash_principal): map hasher(blake2_128_concat) ChainId => CashPrincipalAmount;

        /// The total CASH principal in existence.
        TotalCashPrincipal get(fn total_cash_principal): CashPrincipalAmount;

        /// The total amount supplied per collateral asset.
        TotalSupplyAssets get(fn total_supply_asset): map hasher(blake2_128_concat) ChainAsset => AssetAmount;

        /// The total amount borrowed per collateral asset.
        TotalBorrowAssets get(fn total_borrow_asset): map hasher(blake2_128_concat) ChainAsset => AssetAmount;

        /// The mapping of CASH principal, by account.
        CashPrincipals get(fn cash_principal): map hasher(blake2_128_concat) ChainAccount => CashPrincipal;

        /// The mapping of asset balances, by asset and account.
        AssetBalances get(fn asset_balance): double_map hasher(blake2_128_concat) ChainAsset, hasher(blake2_128_concat) ChainAccount => AssetBalance;

        /// The index of assets with non-zero balance for each account.
        AssetsWithNonZeroBalance get(fn assets_with_non_zero_balance): double_map hasher(blake2_128_concat) ChainAccount, hasher(blake2_128_concat) ChainAsset => ();

        /// The mapping of asset indices, by asset and account.
        LastIndices get(fn last_index): double_map hasher(blake2_128_concat) ChainAsset, hasher(blake2_128_concat) ChainAccount => AssetIndex;

        /// The mapping of notice id to notice.
        Notices get(fn notice): double_map hasher(blake2_128_concat) ChainId, hasher(blake2_128_concat) NoticeId => Option<Notice>;

        /// Notice IDs, indexed by the hash of the notice itself.
        NoticeHashes get(fn notice_hash): map hasher(blake2_128_concat) ChainHash => Option<NoticeId>;

        /// The state of a notice in regards to signing and execution, as tracked by the chain.
        NoticeStates get(fn notice_state): double_map hasher(blake2_128_concat) ChainId, hasher(blake2_128_concat) NoticeId => NoticeState;

        /// The most recent notice emitted for a given chain.
        LatestNotice get(fn latest_notice_id): map hasher(blake2_128_concat) ChainId => Option<(NoticeId, ChainHash)>;

        /// The change authority notices which must be fully signed before we allow notice signing to continue
        NoticeHolds get(fn notice_hold): map hasher(blake2_128_concat) ChainId => Option<NoticeId>;

        /// Index of notices by chain account
        AccountNotices get(fn account_notices): map hasher(blake2_128_concat) ChainAccount => Vec<NoticeId>;

        /// The last used nonce for each account, initialized at zero.
        Nonces get(fn nonce): map hasher(blake2_128_concat) ChainAccount => Nonce;

        /// The asset metadata for each supported asset, which will also be synced with the starports.
        SupportedAssets get(fn asset): map hasher(blake2_128_concat) ChainAsset => Option<AssetInfo>;

        /// Miner of the current block.
        Miner get(fn miner): Option<ChainAccount>;

        /// Mapping of total principal paid to each miner.
        MinerCumulative get(fn miner_cumulative): map hasher(blake2_128_concat) ChainAccount => CashPrincipalAmount;

        /// Validator spread due to miner of last block.
        LastMinerSharePrincipal get(fn last_miner_share_principal): CashPrincipalAmount;

        /// The timestamp of the previous block or defaults to timestamp at genesis.
        LastBlockTimestamp get(fn last_block_timestamp): Timestamp;

        /// The cash index of the previous yield accrual point or defaults to initial cash index.
        LastYieldCashIndex get(fn last_yield_cash_index): CashIndex;

        /// The mapping of ingression queue events, by chain.
        IngressionQueue get(fn ingression_queue): map hasher(blake2_128_concat) ChainId => Option<ChainBlockEvents>;

        /// The mapping of first blocks for which validators are to begin reading events from.
        FirstBlock get(fn first_block): map hasher(blake2_128_concat) ChainId => Option<ChainBlock>;

        /// The mapping of last blocks for which validators added events to the ingression queue, by chain.
        LastProcessedBlock get(fn last_processed_block): map hasher(blake2_128_concat) ChainId => Option<ChainBlock>;

        /// The mapping of worker tallies for each descendant block, on current fork of underlying chain.
        PendingChainBlocks get(fn pending_chain_blocks): map hasher(blake2_128_concat) ChainId => Vec<ChainBlockTally>;

        /// The mapping of worker tallies for each alternate reorg, relative to current fork of underlying chain.
        PendingChainReorgs get(fn pending_chain_reorgs): map hasher(blake2_128_concat) ChainId => Vec<ChainReorgTally>;

        /// Mapping of chain to the relevant Starport address.
        Starports get(fn starports): map hasher(blake2_128_concat) ChainId => Option<ChainStarport>;
    }

    add_extra_genesis {
        config(assets): Vec<AssetInfo>;
        config(validators): Vec<ValidatorKeys>;
        config(starports): Vec<ChainAccount>;
        config(genesis_blocks): Vec<ChainBlock>;
        build(|config| {
            Pallet::<T>::initialize_assets(config.assets.clone());
            Pallet::<T>::initialize_validators(config.validators.clone());
            Pallet::<T>::initialize_starports(config.starports.clone());
            Pallet::<T>::initialize_genesis_blocks(config.genesis_blocks.clone());
        })
    }
}

/* ::EVENTS:: */

decl_event!(
    pub enum Event {
        /// An account has locked an asset. [asset, sender, recipient, amount]
        Locked(ChainAsset, ChainAccount, ChainAccount, AssetAmount),

        /// Revert a lock event while handling a chain re-organization. [asset, sender, recipient, amount]
        ReorgRevertLocked(ChainAsset, ChainAccount, ChainAccount, AssetAmount),

        /// An account has locked CASH. [sender, recipient, principal, index]
        LockedCash(ChainAccount, ChainAccount, CashPrincipalAmount, CashIndex),

        /// Revert a lock cash event while handling a chain re-organization. [sender, recipient, principal, index]
        ReorgRevertLockedCash(ChainAccount, ChainAccount, CashPrincipalAmount, CashIndex),

        /// An account has extracted an asset. [asset, sender, recipient, amount]
        Extract(ChainAsset, ChainAccount, ChainAccount, AssetAmount),

        /// An account has extracted CASH. [sender, recipient, principal, index]
        ExtractCash(ChainAccount, ChainAccount, CashPrincipalAmount, CashIndex),

        /// An account has transferred an asset. [asset, sender, recipient, amount]
        Transfer(ChainAsset, ChainAccount, ChainAccount, AssetAmount),

        /// An account has transferred CASH. [sender, recipient, principal, index]
        TransferCash(ChainAccount, ChainAccount, CashPrincipalAmount, CashIndex),

        /// An account has been liquidated. [asset, collateral_asset, liquidator, borrower, amount]
        Liquidate(
            ChainAsset,
            ChainAsset,
            ChainAccount,
            ChainAccount,
            AssetAmount,
        ),

        /// An account borrowing CASH has been liquidated. [collateral_asset, liquidator, borrower, principal, index]
        LiquidateCash(
            ChainAsset,
            ChainAccount,
            ChainAccount,
            CashPrincipalAmount,
            CashIndex,
        ),

        /// An account using CASH as collateral has been liquidated. [asset, liquidator, borrower, amount]
        LiquidateCashCollateral(ChainAsset, ChainAccount, ChainAccount, AssetAmount),

        /// Miner paid. [miner, principal]
        MinerPaid(ChainAccount, CashPrincipalAmount),

        /// The next code hash has been allowed. [hash]
        AllowedNextCodeHash(CodeHash),

        /// An attempt to set code via hash was made. [hash, result]
        AttemptedSetCodeByHash(CodeHash, dispatch::DispatchResult),

        /// An Ethereum event was successfully processed. [event_id]
        ProcessedChainBlockEvent(ChainBlockEvent),

        /// An Ethereum event failed during processing. [event_id, reason]
        FailedProcessingChainBlockEvent(ChainBlockEvent, Reason),

        /// A new notice is generated by the chain. [notice_id, notice, encoded_notice]
        Notice(NoticeId, Notice, EncodedNotice),

        /// A sequence of governance actions has been executed. [actions]
        ExecutedGovernance(Vec<(Vec<u8>, GovernanceResult)>),

        /// A supported asset has been modified. [asset_info]
        AssetModified(AssetInfo),

        /// A new validator set has been chosen. [validators]
        ChangeValidators(Vec<ValidatorKeys>),

        /// A new yield rate has been chosen. [next_rate, next_start_at]
        SetYieldNext(APR, Timestamp),

        /// Failed to process a given extrinsic. [reason]
        Failure(Reason),
    }
);

/* ::ERRORS:: */

fn check_failure<T: Config>(res: Result<(), Reason>) -> Result<(), Reason> {
    if let Err(err) = res {
        <Module<T>>::deposit_event(Event::Failure(err));
        log!("Cash Failure {:#?}", err);
    }
    res
}

pub trait SessionInterface<AccountId>: frame_system::Config {
    fn has_next_keys(x: AccountId) -> bool;
    fn rotate_session();
}

impl<T: Config> SessionInterface<SubstrateId> for T
where
    T: pallet_session::Config<ValidatorId = SubstrateId>,
{
    fn has_next_keys(x: SubstrateId) -> bool {
        match <pallet_session::Module<T>>::next_keys(x as T::ValidatorId) {
            Some(_keys) => true,
            None => false,
        }
    }

    fn rotate_session() {
        <pallet_session::Module<T>>::rotate_session();
    }
}

impl<T: Config> pallet_session::SessionManager<SubstrateId> for Module<T> {
    // return validator set to use in the next session (aura and grandpa also stage new auths associated w these accountIds)
    fn new_session(session_index: SessionIndex) -> Option<Vec<SubstrateId>> {
        if NextValidators::iter().count() != 0 {
            NextSessionIndex::put(session_index);
            Some(NextValidators::iter().map(|x| x.0).collect::<Vec<_>>())
        } else {
            Some(Validators::iter().map(|x| x.0).collect::<Vec<_>>())
        }
    }

    fn start_session(index: SessionIndex) {
        // if changes have been queued
        // if starting the queued session
        if NextSessionIndex::get() == index && NextValidators::iter().count() != 0 {
            // delete existing validators
            for validator in <Validators>::iter_values() {
                <Validators>::take(&validator.substrate_id);
            }
            // push next validators into current validators
            for (id, validator) in <NextValidators>::iter() {
                <NextValidators>::take(&id);
                <Validators>::insert(&id, validator);
            }
        } else {
            ()
        }
    }
    fn end_session(_: SessionIndex) {
        ()
    }
}

/*
-- Block N --
changeAuth extrinsic, nextValidators set, hold is set, rotate_session is called
* new_session returns the nextValidators

-- Afterwards --
"ShouldEndSession" returns true when notice era notices were signed
* when it does, start_session sets Validators = NextValidators

*/

fn vec_to_set<T: Ord + Debuggable>(a: Vec<T>) -> BTreeSet<T> {
    let mut a_set = BTreeSet::<T>::new();
    for v in a {
        a_set.insert(v);
    }
    return a_set;
}

fn has_requisite_signatures(notice_state: NoticeState, validators: &Vec<ValidatorKeys>) -> bool {
    match notice_state {
        NoticeState::Pending { signature_pairs } => match signature_pairs {
            ChainSignatureList::Eth(signature_pairs) => {
                // Note: inefficient, probably best to store as sorted lists / zip compare
                type EthAddrType = <chains::Ethereum as chains::Chain>::Address;
                let signature_set =
                    vec_to_set::<EthAddrType>(signature_pairs.iter().map(|p| p.0).collect());
                let validator_set =
                    vec_to_set::<EthAddrType>(validators.iter().map(|v| v.eth_address).collect());
                chains::has_super_majority::<EthAddrType>(&signature_set, &validator_set)
            }
            _ => false,
        },
        _ => false,
    }
}

// periodic except when new authorities are pending and when an era notice has just been completed
impl<T: Config> pallet_session::ShouldEndSession<T::BlockNumber> for Module<T> {
    fn should_end_session(now: T::BlockNumber) -> bool {
        if NextValidators::iter().count() > 0 {
            // Check if we should end the hold
            let validators: Vec<_> = Validators::iter().map(|v| v.1).collect();
            let every_notice_hold_executed = NoticeHolds::iter().all(|(chain_id, notice_id)| {
                has_requisite_signatures(NoticeStates::get(chain_id, notice_id), &validators)
            });

            if every_notice_hold_executed {
                for (chain_id, _) in NoticeHolds::iter() {
                    NoticeHolds::take(chain_id);
                }
                log!("should_end_session=true[next_validators]");
                true
            } else {
                log!("should_end_session=false[pending_notice_held]");
                false
            }
        } else {
            // no era changes pending, periodic
            let period: T::BlockNumber = <T>::BlockNumber::from(params::SESSION_PERIOD as u32);
            let is_new_period = (now % period) == <T>::BlockNumber::from(0 as u32);

            if is_new_period {
                log!(
                    "should_end_session={}[periodic {:?}%{:?}]",
                    is_new_period,
                    now,
                    period
                );
            }
            is_new_period
        }
    }
}

impl<T: Config> frame_support::traits::EstimateNextSessionRotation<T::BlockNumber> for Module<T> {
    fn average_session_length() -> T::BlockNumber {
        T::BlockNumber::zero()
    }

    fn estimate_current_session_progress(now: T::BlockNumber) -> (Option<Percent>, Weight) {
        let period: T::BlockNumber = <T>::BlockNumber::from(params::SESSION_PERIOD as u32);
        (
            Some(Percent::from_rational(now % period, period)),
            Weight::zero(),
        )
    }

    fn estimate_next_session_rotation(now: T::BlockNumber) -> (Option<T::BlockNumber>, Weight) {
        let period: T::BlockNumber = <T>::BlockNumber::from(params::SESSION_PERIOD as u32);
        (Some(now + period - now % period), Weight::zero())
    }
}

fn get_exec_req_weights<T: Config>(request: Vec<u8>) -> frame_support::weights::Weight {
    let request_str = match str::from_utf8(&request[..]).map_err(|_| Reason::InvalidUTF8) {
        Err(_) => return params::ERROR_WEIGHT,
        Ok(f) => f,
    };
    match trx_request::parse_request(request_str) {
        Ok(trx_request::TrxRequest::Extract(_max_amount, _asset, _account)) => {
            <T as Config>::WeightInfo::exec_trx_request_extract()
        }

        Ok(trx_request::TrxRequest::Transfer(_max_amount, _asset, _account)) => {
            <T as Config>::WeightInfo::exec_trx_request_transfer()
        }

        Ok(trx_request::TrxRequest::Liquidate(_max_amount, _borrowed, _collat, _account)) => {
            <T as Config>::WeightInfo::exec_trx_request_liquidate()
        }

        _ => params::ERROR_WEIGHT,
    }
}

fn get_chain_reorg_weights<T: Config>(
    reorg: &ChainReorg,
    signature: &ChainSignature,
) -> Result<frame_support::weights::Weight, Reason> {
    match reorg {
        ChainReorg::Eth {
            from_hash: _,
            to_hash: _,
            forward_blocks,
            reverse_blocks,
        } => {
            let forward_event_count: u64 = forward_blocks
                .iter()
                .fold(0usize, |acc, x| acc.checked_add(x.events.len()).unwrap())
                .try_into()
                .unwrap();
            let reverse_event_count: u64 = reverse_blocks
                .iter()
                .fold(0usize, |acc, x| acc.checked_add(x.events.len()).unwrap())
                .try_into()
                .unwrap();
            let event_count = forward_event_count + reverse_event_count;

            if let Some(prior) = PendingChainReorgs::get(ChainId::Eth)
                .iter_mut()
                .find(|r| r.reorg == *reorg)
            {
                let validator = core::recover_validator::<T>(&reorg.encode(), *signature)?;
                // if reorg would get applied, just estimate gas by counting the number of reorged events
                if prior.would_have_enough_support(&prior.support, &validator) {
                    let avg_weight = <T as Config>::WeightInfo::exec_trx_request_extract();
                    // TODO: only count forward weight if we have passed than min_event_blocks
                    Ok(event_count * avg_weight)
                } else {
                    Ok(<T as Config>::WeightInfo::receive_chain_reorg_pending(
                        event_count.try_into().unwrap(),
                    ))
                }
            } else {
                Ok(<T as Config>::WeightInfo::receive_chain_reorg_pending(
                    event_count.try_into().unwrap(),
                ))
            }
        }
    }
}

/* ::MODULE:: */
/* ::EXTRINSICS:: */

// Dispatch Extrinsic Lifecycle //

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        fn on_runtime_upgrade() -> Weight {
            0 // XXX
        }

        /// Called by substrate on block initialization.
        /// Our initialization function is fallible, but that's not allowed.
        fn on_initialize(block: T::BlockNumber) -> frame_support::weights::Weight {
            match internal::initialize::on_initialize::<T>() {
                Ok(()) => <T as Config>::WeightInfo::on_initialize(SupportedAssets::iter().count().try_into().unwrap()),
                Err(err) => {
                    // This should never happen...
                    error!("Could not initialize block!!! {:#?} {:#?}", block, err);
                    0
                }
            }
        }

        /// Offchain Worker entry point.
        fn offchain_worker(block_number: T::BlockNumber) {
            match internal::events::track_chain_events::<T>() {
                Ok(()) => (),
                Err(Reason::WorkerBusy) => {
                    debug!("offchain_worker is still busy in track_chain_events");
                }
                Err(err) => {
                    error!("offchain_worker error during track_chain_events: {:?}", err);
                }
            }

            // XXX we need to 'lock' notices too right?
            match internal::notices::process_notices::<T>(block_number) {
                (succ, skip, failures) => {
                    if succ > 0 || skip > 0 {
                        log!("offchain_worker process_notices: {} successful, {} skipped", succ, skip);
                    }
                    if failures.len() > 0 {
                        error!("offchain_worker error(s) during process notices: {:?}", failures);
                    }
                }
            }
        }

        /// Sets the miner of the this block via inherent
        #[weight = (0, DispatchClass::Operational)]
        fn set_miner(origin, miner: ChainAccount) {
            ensure_none(origin)?;
            internal::miner::set_miner::<T>(miner);
        }

        /// Sets the keys for the next set of validators beginning at the next session. [Root]
        #[weight = (<T as Config>::WeightInfo::change_validators(), DispatchClass::Operational, Pays::No)]
        pub fn change_validators(origin, validators: Vec<ValidatorKeys>) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::change_validators::change_validators::<T>(validators))?)
        }

        /// Sets the allowed next code hash to the given hash. [Root]
        #[weight = (<T as Config>::WeightInfo::allow_next_code_with_hash(), DispatchClass::Operational, Pays::No)]
        pub fn allow_next_code_with_hash(origin, hash: CodeHash) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::next_code::allow_next_code_with_hash::<T>(hash))?)
        }

        /// Sets the allowed next code hash to the given hash. [User] [Free]
        #[weight = (
            <T as Config>::WeightInfo::set_next_code_via_hash(code.len().try_into().unwrap_or(u32::MAX)),
            DispatchClass::Operational,
            Pays::No
        )]
        pub fn set_next_code_via_hash(origin, code: Vec<u8>) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            let res = check_failure::<T>(internal::next_code::set_next_code_via_hash::<T>(code));
            log!("Set next code via hash result: {:?}", res);
            Ok(res?)
        }

        #[weight = (0, DispatchClass::Operational, Pays::No)]
        pub fn set_starport(origin, starport: ChainStarport) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            log!("Setting Starport to {:?}", starport);
            Starports::insert(starport.chain_id(), starport);
            Ok(())
        }

        #[weight = (0, DispatchClass::Operational, Pays::No)]
        pub fn set_genesis_block(origin, chain_block: ChainBlock) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            log!("Setting last processed block to {:?}", chain_block);
            FirstBlock::insert(chain_block.chain_id(), chain_block.clone());
            LastProcessedBlock::insert(chain_block.chain_id(), chain_block);
            Ok(())
        }

        /// Sets the supply cap for a given chain asset [Root]
        #[weight = (<T as Config>::WeightInfo::set_supply_cap(), DispatchClass::Operational, Pays::No)]
        pub fn set_supply_cap(origin, asset: ChainAsset, amount: AssetAmount) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::supply_cap::set_supply_cap::<T>(asset, amount))?)
        }

        /// Set the liquidity factor for an asset [Root]
        #[weight = (<T as Config>::WeightInfo::set_liquidity_factor(), DispatchClass::Operational, Pays::No)]
        pub fn set_liquidity_factor(origin, asset: ChainAsset, factor: LiquidityFactor) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::assets::set_liquidity_factor::<T>(asset, factor))?)
        }

        /// Update the interest rate model for a given asset. [Root]
        #[weight = (<T as Config>::WeightInfo::set_rate_model(), DispatchClass::Operational, Pays::No)]
        pub fn set_rate_model(origin, asset: ChainAsset, model: InterestRateModel) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::assets::set_rate_model::<T>(asset, model))?)
        }

        /// Set the cash yield rate at some point in the future. [Root]
        #[weight = (<T as Config>::WeightInfo::set_yield_next(), DispatchClass::Operational, Pays::No)]
        pub fn set_yield_next(origin, next_apr: APR, next_apr_start: Timestamp) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::set_yield_next::set_yield_next::<T>(next_apr, next_apr_start))?)
        }

        /// Adds the asset to the runtime by defining it as a supported asset. [Root]
        #[weight = (<T as Config>::WeightInfo::support_asset(), DispatchClass::Operational, Pays::No)]
        pub fn support_asset(origin, asset_info: AssetInfo) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::assets::support_asset::<T>(asset_info))?)
        }

        /// Receive the chain blocks message from the worker to make progress on event ingression. [Root]
        #[weight = (0, DispatchClass::Operational, Pays::No)]
        pub fn receive_chain_blocks(origin, blocks: ChainBlocks, signature: ChainSignature) -> dispatch::DispatchResult {
            log!("receive_chain_blocks(origin, blocks, signature): {:?} {:?}", blocks, signature);
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::events::receive_chain_blocks::<T>(blocks, signature))?)
        }

        /// Receive the chain blocks message from the worker to make progress on event ingression. [Root]
        #[weight = (get_chain_reorg_weights::<T>(reorg, signature).unwrap_or(params::ERROR_WEIGHT), DispatchClass::Operational, Pays::No)]
        pub fn receive_chain_reorg(origin, reorg: ChainReorg, signature: ChainSignature) -> dispatch::DispatchResult {
            log!("receive_chain_reorg(origin, reorg, signature): {:?} {:?}", reorg, signature);
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::events::receive_chain_reorg::<T>(reorg, signature))?)
        }

        #[weight = (<T as Config>::WeightInfo::publish_signature(), DispatchClass::Operational, Pays::No)]
        pub fn publish_signature(origin, chain_id: ChainId, notice_id: NoticeId, signature: ChainSignature) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::notices::publish_signature::<T>(chain_id, notice_id, signature))?)
        }

        /// Execute a transaction request on behalf of a user
        #[weight = (get_exec_req_weights::<T>(request.to_vec()), DispatchClass::Normal, Pays::No)]
        pub fn exec_trx_request(origin, request: Vec<u8>, signature: ChainAccountSignature, nonce: Nonce) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::exec_trx_request::exec::<T>(request, signature, nonce))?)
        }
    }
}

/// Reading error messages inside `decl_module!` can be difficult, so we move them here.
impl<T: Config> Module<T> {
    /// Initializes the set of supported assets from a config value.
    fn initialize_assets(assets: Vec<AssetInfo>) {
        for asset in assets {
            SupportedAssets::insert(&asset.asset, asset);
        }
    }

    /// Set the initial set of validators from the genesis config.
    /// NextValidators will become current Validators upon first session start.
    fn initialize_validators(validators: Vec<ValidatorKeys>) {
        if validators.is_empty() {
            warn!("Validators must be set in the genesis config");
        }
        for validator in validators {
            // Note: See pipeline commit for usage of T::AccountStore
            log!("Adding validator: {:?}", validator);
            <Validators>::insert(&validator.substrate_id, validator.clone());
            assert!(T::AccountStore::insert(&validator.substrate_id, ()).is_ok());
        }
    }

    /// Set the initial starports from the genesis config.
    fn initialize_starports(starports: Vec<ChainStarport>) {
        if starports.is_empty() {
            warn!("Starports must be set in the genesis config");
        }
        for starport in starports {
            log!("Adding Starport {:?}", starport);
            assert!(
                Starports::get(starport.chain_id()) == None,
                "Duplicate chain starport in genesis config"
            );
            Starports::insert(starport.chain_id(), starport);
        }
    }

    /// Set the initial last processed blocks from the genesis config.
    fn initialize_genesis_blocks(genesis_blocks: Vec<ChainBlock>) {
        if genesis_blocks.is_empty() {
            warn!("Genesis blocks must be set in the genesis config");
        }
        for genesis_block in genesis_blocks {
            log!("Adding Genesis Block {:?}", genesis_block);
            assert!(
                FirstBlock::get(genesis_block.chain_id()) == None,
                "Duplicate genesis block in genesis config"
            );
            FirstBlock::insert(genesis_block.chain_id(), genesis_block.clone());
            LastProcessedBlock::insert(genesis_block.chain_id(), genesis_block);
        }
    }

    // ** API / View Functions ** //

    /// Get the asset balance for the given account.
    pub fn get_account_balance(
        account: ChainAccount,
        asset: ChainAsset,
    ) -> Result<AssetBalance, Reason> {
        Ok(core::get_account_balance::<T>(account, asset)?)
    }

    /// Get the asset info for the given asset.
    pub fn get_asset(asset: ChainAsset) -> Result<AssetInfo, Reason> {
        Ok(internal::assets::get_asset::<T>(asset)?)
    }

    /// Get the cash yield.
    pub fn get_cash_yield() -> Result<APR, Reason> {
        Ok(core::get_cash_yield::<T>()?)
    }

    /// Get the cash data.
    pub fn get_cash_data() -> Result<(CashIndex, CashPrincipal, Balance), Reason> {
        let (cash_index, cash_principal_amount) = core::get_cash_data::<T>()?;
        let cash_principal: CashPrincipal = cash_principal_amount.try_into()?;
        let total_cash = cash_index.cash_balance(cash_principal)?;
        Ok((cash_index, cash_principal, total_cash))
    }

    /// Get the full cash balance for the given account.
    pub fn get_full_cash_balance(account: ChainAccount) -> Result<AssetBalance, Reason> {
        Ok(core::get_cash_balance_with_asset_interest::<T>(account)?.value)
    }

    /// Get the liquidity for the given account.
    pub fn get_liquidity(account: ChainAccount) -> Result<AssetBalance, Reason> {
        Ok(core::get_liquidity::<T>(account)?.value)
    }

    /// Get the total supply for the given asset.
    pub fn get_market_totals(asset: ChainAsset) -> Result<(AssetAmount, AssetAmount), Reason> {
        Ok(core::get_market_totals::<T>(asset)?)
    }

    /// Get the rates for the given asset.
    pub fn get_rates(asset: ChainAsset) -> Result<(APR, APR), Reason> {
        Ok(internal::assets::get_rates::<T>(asset)?)
    }

    /// Get the list of assets
    pub fn get_assets() -> Result<Vec<AssetInfo>, Reason> {
        Ok(internal::assets::get_assets::<T>()?)
    }

    /// Get the a list of all chain accounts
    pub fn get_accounts() -> Result<Vec<ChainAccount>, Reason> {
        Ok(core::get_accounts::<T>()?)
    }

    /// Get the user counts for the given asset.
    pub fn get_asset_meta(
    ) -> Result<(BTreeMap<String, u32>, BTreeMap<String, u32>, u32, u32), Reason> {
        Ok(core::get_asset_meta::<T>()?)
    }

    /// Get the all liquidity
    pub fn get_accounts_liquidity() -> Result<Vec<(ChainAccount, String)>, Reason> {
        let accounts: Vec<(ChainAccount, String)> = core::get_accounts_liquidity::<T>()?
            .iter()
            .map(|(chain_account, bal)| (chain_account.clone(), format!("{}", bal)))
            .collect();
        Ok(accounts)
    }

    /// Get the portfolio for the given chain account.
    pub fn get_portfolio(account: ChainAccount) -> Result<Portfolio, Reason> {
        Ok(core::get_portfolio::<T>(account)?)
    }

    /// Get the active validators, and  sets
    pub fn get_validator_info() -> Result<(Vec<ValidatorKeys>, Vec<(ChainAccount, String)>), Reason>
    {
        let validator_keys: Vec<ValidatorKeys> = Validators::iter().map(|(_, v)| v).collect();
        let (cash_index, _) = core::get_cash_data::<T>()?;

        let miner_earnings: Vec<(ChainAccount, String)> = MinerCumulative::iter()
            .map(|(miner_address, miner_principal_amount)| {
                let miner_principal: CashPrincipal = miner_principal_amount
                    .try_into()
                    .unwrap_or(CashPrincipal::from_nominal("0"));
                let miner_balance = cash_index
                    .cash_balance(miner_principal)
                    .unwrap_or(Balance::from_nominal("0", CASH))
                    .value
                    .to_string();
                (miner_address, miner_balance)
            })
            .collect();
        Ok((validator_keys, miner_earnings))
    }
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    /// Validate unsigned call to this module.
    ///
    /// By default unsigned transactions are disallowed, but implementing the validator
    /// here we make sure that some particular calls (the ones produced by offchain worker)
    /// are being whitelisted and marked as valid.
    fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        internal::validate_trx::check_validation_failure(
            call,
            internal::validate_trx::validate_unsigned::<T>(source, call),
        )
        .unwrap_or(InvalidTransaction::Call.into())
    }
}
