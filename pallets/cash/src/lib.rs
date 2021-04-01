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
        ChainAccount, ChainAccountSignature, ChainAsset, ChainHash, ChainId, ChainSignature,
        ChainSignatureList,
    },
    events::{ChainLogEvent, ChainLogId, EventState},
    notices::{Notice, NoticeId, NoticeState},
    portfolio::Portfolio,
    types::{
        AssetAmount, AssetBalance, AssetIndex, AssetInfo, Bips, CashIndex, CashPrincipal,
        CashPrincipalAmount, CodeHash, EncodedNotice, GovernanceResult, InterestRateModel,
        LiquidityFactor, Nonce, Reason, SessionIndex, Timestamp, ValidatorKeys, ValidatorSig, APR,
    },
};

use codec::alloc::string::String;
use frame_support::{
    decl_event, decl_module, decl_storage, dispatch,
    sp_runtime::traits::Convert,
    traits::{StoredMap, UnfilteredDispatchable},
    weights::{DispatchClass, GetDispatchInfo, Pays, Weight},
    Parameter,
};
use frame_system;
use frame_system::{ensure_none, ensure_root, offchain::CreateSignedTransaction};
use our_std::{collections::btree_set::BTreeSet, error, log, str, vec::Vec, Debuggable};
use sp_core::crypto::AccountId32;
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionSource, TransactionValidity,
};

use pallet_oracle;
use pallet_oracle::ticker::Ticker;
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
pub mod portfolio;
pub mod rates;
pub mod reason;
pub mod serdes;
pub mod symbol;
pub mod trx_req;
pub mod types;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

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
        NextValidators get(fn next_validators) : map hasher(blake2_128_concat) SubstrateId => Option<ValidatorKeys>;

        /// The current set of allowed validators, and their associated keys.
        Validators get(fn validators) : map hasher(blake2_128_concat) SubstrateId => Option<ValidatorKeys>;

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

        /// The mapping of (status of) events witnessed on a given chain, by event id.
        EventStates get(fn event_state): map hasher(blake2_128_concat) ChainLogId => EventState;

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

        /// Mapping of strings to tickers (valid tickers indexed by ticker string).
        Tickers get(fn ticker): map hasher(blake2_128_concat) String => Option<Ticker>;

        /// Miner of the current block.
        Miner get(fn miner): Option<ChainAccount>;

        /// Validator spread due to miner of last block.
        LastMinerSharePrincipal get(fn last_miner_share_principal): CashPrincipalAmount;

        /// The timestamp of the previous block or defaults to timestamp at genesis.
        LastBlockTimestamp get(fn last_block_timestamp): Timestamp;

        /// The cash index of the previous yield accrual point or defaults to initial cash index.
        LastYieldCashIndex get(fn last_yield_cash_index): CashIndex;
    }
    add_extra_genesis {
        config(assets): Vec<AssetInfo>;
        config(validators): Vec<ValidatorKeys>;
        build(|config| {
            Module::<T>::initialize_assets(config.assets.clone());
            Module::<T>::initialize_validators(config.validators.clone());
        })
    }
}

/* ::EVENTS:: */

decl_event!(
    pub enum Event {
        /// An account has locked an asset. [asset, sender, recipient, amount]
        Locked(ChainAsset, ChainAccount, ChainAccount, AssetAmount),

        /// An account has locked CASH. [sender, recipient, principal, index]
        LockedCash(ChainAccount, ChainAccount, CashPrincipalAmount, CashIndex),

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

        /// The next code hash has been allowed. [hash]
        AllowedNextCodeHash(CodeHash),

        /// An attempt to set code via hash was made. [hash, result]
        AttemptedSetCodeByHash(CodeHash, dispatch::DispatchResult),

        /// An Ethereum event was successfully processed. [event_id]
        ProcessedChainEvent(ChainLogId),

        /// An Ethereum event failed during processing. [event_id, reason]
        FailedProcessingChainEvent(ChainLogId, Reason),

        /// A new notice is generated by the chain. [notice_id, notice, encoded_notice]
        Notice(NoticeId, Notice, EncodedNotice),

        /// A notice has been signe. [chain_id, notice_id, message, signatures]
        SignedNotice(ChainId, NoticeId, EncodedNotice, ChainSignatureList),

        /// A sequence of governance actions has been executed. [actions]
        ExecutedGovernance(Vec<(Vec<u8>, GovernanceResult)>),

        /// A new supply cap has been set. [asset, cap]
        SetSupplyCap(ChainAsset, AssetAmount),

        /// A new validator set has been chosen
        ChangeValidators(Vec<ValidatorKeys>),

        /// A new yield rate has been chosen
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
            for kv in <Validators>::iter() {
                <Validators>::take(&kv.0);
            }
            // push next validators into current validators
            for (id, chain_keys) in <NextValidators>::iter() {
                <NextValidators>::take(&id);
                <Validators>::insert(&id, chain_keys);
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

fn intersection_count<T: Ord + Debuggable>(a: Vec<T>, b: Vec<T>) -> usize {
    let mut a_set = BTreeSet::<T>::new();
    for v in a {
        a_set.insert(v);
    }

    let mut b_set = BTreeSet::<T>::new();
    for v in b {
        b_set.insert(v);
    }

    a_set.intersection(&b_set).into_iter().count()
}

fn has_requisite_signatures(notice_state: NoticeState, validators: &Vec<ValidatorKeys>) -> bool {
    let validator_count = validators.iter().count();
    let quorum_count = validator_count; // TODO: Add a real quorum count

    match notice_state {
        NoticeState::Pending { signature_pairs } => match signature_pairs {
            ChainSignatureList::Eth(signature_pairs) => {
                intersection_count(
                    signature_pairs.iter().map(|p| p.0).collect(),
                    validators.iter().map(|v| v.eth_address).collect(),
                ) >= quorum_count
            }
            _ => false,
        },
        _ => false,
    }
}

// XXX should this have all these printlns?
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
                println!("should_end_session=true[next_validators]");
                true
            } else {
                println!("should_end_session=false[pending_notice_held]");
                false
            }
        } else {
            // no era changes pending, periodic
            let period: T::BlockNumber = <T>::BlockNumber::from(params::SESSION_PERIOD as u32);
            let is_new_period = (now % period) == <T>::BlockNumber::from(0 as u32);

            if is_new_period {
                println!(
                    "should_end_session={}[periodic {:?}%{:?}]",
                    is_new_period, now, period
                );
            }
            is_new_period
        }
    }
}

impl<T: Config> frame_support::traits::EstimateNextSessionRotation<T::BlockNumber> for Module<T> {
    fn estimate_next_session_rotation(now: T::BlockNumber) -> Option<T::BlockNumber> {
        let period: T::BlockNumber = <T>::BlockNumber::from(params::SESSION_PERIOD as u32);
        Some(now + period - now % period)
    }

    // The validity of this weight depends on the implementation of `estimate_next_session_rotation`
    fn weight(_now: T::BlockNumber) -> Weight {
        0
    }
}

/* ::MODULE:: */
/* ::EXTRINSICS:: */

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        /// Called by substrate on block initialization.
        /// Our initialization function is fallible, but that's not allowed.
        fn on_initialize(block: T::BlockNumber) -> frame_support::weights::Weight {
            match core::on_initialize::<T>() {
                Ok(weight) => weight,
                Err(err) => {
                    // This should never happen...
                    error!("Could not initialize block!!! {:#?} {:#?}", block, err);
                    0
                }
            }
        }

        /// Sets the miner of the this block via inherent
        #[weight = (
            0,
            DispatchClass::Operational
        )]
        fn set_miner(origin, miner: ChainAccount) {
            ensure_none(origin)?;

            log!("set_miner({:?})", miner);
            Miner::put(miner);
        }

        /// Sets the keys for the next set of validators beginning at the next session. [Root]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn change_validators(origin, validators: Vec<ValidatorKeys>) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::change_validators::change_validators::<T>(validators))?)
        }

        /// Sets the allowed next code hash to the given hash. [Root]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn allow_next_code_with_hash(origin, hash: CodeHash) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::next_code::allow_next_code_with_hash::<T>(hash))?)
        }

        /// Sets the allowed next code hash to the given hash. [User] [Free]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn set_next_code_via_hash(origin, code: Vec<u8>) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::next_code::set_next_code_via_hash::<T>(code))?)
        }

        /// Sets the supply cap for a given chain asset [Root]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn set_supply_cap(origin, asset: ChainAsset, amount: AssetAmount) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::supply_cap::set_supply_cap::<T>(asset, amount))?)
        }

        /// Set the liquidity factor for an asset [Root]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn set_liquidity_factor(origin, asset: ChainAsset, factor: LiquidityFactor) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::assets::set_liquidity_factor::<T>(asset, factor))?)
        }

        /// Update the interest rate model for a given asset. [Root]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn set_rate_model(origin, asset: ChainAsset, model: InterestRateModel) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::assets::set_rate_model::<T>(asset, model))?)
        }

        /// Set the cash yield rate at some point in the future. [Root]
        #[weight = (<T as Config>::WeightInfo::set_yield_next(), DispatchClass::Operational, Pays::No)] // XXX
        pub fn set_yield_next(origin, next_apr: APR, next_apr_start: Timestamp) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::set_yield_next::set_yield_next::<T>(next_apr, next_apr_start))?)
        }

        /// Adds the asset to the runtime by defining it as a supported asset. [Root]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn support_asset(origin, asset: ChainAsset, asset_info: AssetInfo) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Ok(check_failure::<T>(internal::assets::support_asset::<T>(asset, asset_info))?)
        }

        // TODO: Do we need to sign the event id, too?
        #[weight = (<T as Config>::WeightInfo::receive_event(), DispatchClass::Operational, Pays::No)] // XXX
        pub fn receive_event(origin, event_id: ChainLogId, event: ChainLogEvent, signature: ValidatorSig) -> dispatch::DispatchResult { // XXX sig
            log!("receive_event(origin,event_id,event,signature): {:?} {:?} {}", event_id, &event, hex::encode(&signature)); // XXX ?
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::events::receive_event::<T>(event_id, event, signature))?)
        }

        #[weight = (<T as Config>::WeightInfo::publish_signature(), DispatchClass::Operational, Pays::No)]
        pub fn publish_signature(origin, chain_id: ChainId, notice_id: NoticeId, signature: ChainSignature) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::notices::publish_signature(chain_id, notice_id, signature))?)
        }

        /// Offchain Worker entry point.
        fn offchain_worker(block_number: T::BlockNumber) {
            if let Err(e) = internal::events::fetch_events::<T>() {
                log!("offchain_worker error during fetch events: {:?}", e);
            }

            let (succ, skip, failures) = internal::notices::process_notices::<T>(block_number);

            let fail: usize = failures.iter().count();

            if succ > 0 || skip > 0 || fail > 0 {
                log!("offchain_worker process_notices: {} succ, {} skip, {} fail", succ, skip, fail);
            }

            if fail > 0 {
                log!("offchain_worker error(s) during process notices: {:?}", failures);
            }
        }

        /// Execute a transaction request on behalf of a user
        #[weight = (1, DispatchClass::Normal, Pays::No)] // XXX
        pub fn exec_trx_request(origin, request: Vec<u8>, signature: ChainAccountSignature, nonce: Nonce) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::exec_trx_request::exec::<T>(request, signature, nonce))?)
        }

        // Remove any notice holds if they have been executed
        #[weight = (1, DispatchClass::Normal, Pays::No)] // XXX
        pub fn cull_notices(origin) -> dispatch::DispatchResult {
            log!("Culling executed notices");
            NoticeHolds::iter().for_each(|(chain_id, notice_id)| {
                match NoticeStates::get(chain_id, notice_id) {
                    NoticeState::Executed => {
                        NoticeHolds::take(chain_id);
                        log!("Removed notice hold {:?}:{:?} as it was already executed", chain_id, notice_id);
                    },
                    _ => ()
                }
            });
            Ok(())
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
        assert!(
            !validators.is_empty(),
            "Validators must be set in the genesis config"
        );
        for validator_keys in validators {
            log!("Adding validator with keys: {:?}", validator_keys);
            assert!(
                <Validators>::get(&validator_keys.substrate_id) == None,
                "Duplicate validator keys in genesis config"
            );
            // XXX for Substrate 3
            // assert!(
            //     T::AccountStore::insert(&validator_keys.substrate_id, ()).is_ok(),
            //     "Could not placate the substrate account existence thing"
            // );
            <Validators>::insert(&validator_keys.substrate_id, validator_keys.clone());
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
        Ok(core::get_asset::<T>(asset)?)
    }

    /// Get the cash yield.
    pub fn get_cash_yield() -> Result<APR, Reason> {
        Ok(core::get_cash_yield::<T>()?)
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
        Ok(core::get_rates::<T>(asset)?)
    }

    /// Get the rates for the given asset.
    pub fn get_accounts() -> Result<Vec<ChainAccount>, Reason> {
        Ok(core::get_accounts::<T>()?)
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
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    /// Validate unsigned call to this module.
    ///
    /// By default unsigned transactions are disallowed, but implementing the validator
    /// here we make sure that some particular calls (the ones produced by offchain worker)
    /// are being whitelisted and marked as valid.
    fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        internal::validate_trx::validate_unsigned::<T>(source, call)
            .unwrap_or(InvalidTransaction::Call.into())
    }
}
