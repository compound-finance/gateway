#![allow(incomplete_features)]
#![feature(array_methods)]
#![feature(associated_type_defaults)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(const_panic)]
#![feature(destructuring_assignment)]
#![feature(str_split_once)]

#[macro_use]
extern crate alloc;
extern crate ethereum_client;
extern crate trx_request;

use crate::chains::{
    ChainAccount, ChainAccountSignature, ChainAsset, ChainHash, ChainId, ChainSignature,
    ChainSignatureList,
};
use crate::events::{ChainLogEvent, ChainLogId, EventState};
use crate::notices::{Notice, NoticeId, NoticeState};
use crate::rates::{InterestRateModel, APR};
use crate::reason::Reason;
use crate::symbol::Ticker;
use crate::types::{
    AssetAmount, AssetBalance, AssetIndex, AssetInfo, AssetPrice, Bips, CashIndex, CashPrincipal,
    CashPrincipalAmount, CodeHash, EncodedNotice, LiquidityFactor, Nonce, ReporterSet,
    SessionIndex, Timestamp, ValidatorKeys, ValidatorSig,
};
use codec::alloc::string::String;
use frame_support::{
    decl_event, decl_module, decl_storage, dispatch,
    traits::StoredMap,
    weights::{DispatchClass, Pays},
    Parameter,
};
use frame_system::{ensure_none, ensure_root, offchain::CreateSignedTransaction};
use our_std::{str, vec::Vec};
use sp_core::crypto::AccountId32;
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
};

use frame_support::sp_runtime::traits::Convert;
use frame_support::{traits::UnfilteredDispatchable, weights::GetDispatchInfo};

use pallet_session;
use pallet_timestamp;

#[macro_use]
extern crate lazy_static;

pub mod chains;
pub mod converters;
pub mod core;
pub mod events;
pub mod factor;
pub mod internal;
pub mod log;
pub mod notices;
pub mod params;
pub mod portfolio;
pub mod rates;
pub mod reason;
pub mod serdes;
pub mod symbol;
pub mod trx_req;
pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod testdata;

/// Type for linking sessions to validators.
pub type SubstrateId = AccountId32;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config:
    frame_system::Config + CreateSignedTransaction<Call<Self>> + pallet_timestamp::Config
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

        /// Notices contain information which can be synced to Starports
        Notices get(fn notice): double_map hasher(blake2_128_concat) ChainId, hasher(blake2_128_concat) NoticeId => Option<Notice>;

        /// Notice IDs, indexed by the hash of the notice itself
        NoticeHashes get(fn notice_hash): map hasher(blake2_128_concat) ChainHash => Option<NoticeId>;

        /// The state of a notice in regards to signing and execution, as tracked by the chain
        NoticeStates get(fn notice_state): double_map hasher(blake2_128_concat) ChainId, hasher(blake2_128_concat) NoticeId => NoticeState;

        /// The most recent notice emitted for a given chain
        LatestNotice get(fn latest_notice_id): map hasher(blake2_128_concat) ChainId => Option<(NoticeId, ChainHash)>;

        /// Index of notices by chain account
        AccountNotices get(fn account_notices): map hasher(blake2_128_concat) ChainAccount => Vec<NoticeId>;

        /// The last used nonce for each account, initialized at zero.
        Nonces get(fn nonce): map hasher(blake2_128_concat) ChainAccount => Nonce;

        /// Mapping of latest prices for each price ticker.
        Prices get(fn price): map hasher(blake2_128_concat) Ticker => Option<AssetPrice>;

        /// Mapping of assets to the last time their price was updated.
        PriceTimes get(fn price_time): map hasher(blake2_128_concat) Ticker => Option<Timestamp>;

        /// Ethereum addresses of open oracle price reporters.
        PriceReporters get(fn reporters): ReporterSet; // XXX if > 1, how are we combining?

        /// The asset metadata for each supported asset, which will also be synced with the starports.
        SupportedAssets get(fn asset): map hasher(blake2_128_concat) ChainAsset => Option<AssetInfo>;

        /// Mapping of strings to tickers (valid tickers indexed by ticker string).
        Tickers get(fn ticker): map hasher(blake2_128_concat) String => Option<Ticker>;

        /// Miner of the current block.
        Miner get(fn miner): Option<ChainAccount>;

        /// Validator spread due to miner of last block.
        LastMinerSharePrincipal get(fn last_miner_share_principal): CashPrincipalAmount;
    }
    add_extra_genesis {
        config(assets): Vec<AssetInfo>;
        config(reporters): ReporterSet;
        config(validators): Vec<ValidatorKeys>;
        build(|config| {
            Module::<T>::initialize_assets(config.assets.clone());
            Module::<T>::initialize_reporters(config.reporters.clone());
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
        ExecutedGovernance(Vec<(Vec<u8>, bool)>),

        /// A new supply cap has been set. [asset, cap]
        SetSupplyCap(ChainAsset, AssetAmount),

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
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
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

        /// Set the price using the open price feed. [User] [Free]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn post_price(origin, payload: Vec<u8>, signature: Vec<u8>) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::oracle::post_price::<T>(payload, signature))?)
        }

        // TODO: Do we need to sign the event id, too?
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn receive_event(origin, event_id: ChainLogId, event: ChainLogEvent, signature: ValidatorSig) -> dispatch::DispatchResult { // XXX sig
            log!("receive_event(origin,event_id,event,signature): {:?} {:?} {}", event_id, &event, hex::encode(&signature)); // XXX ?
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::events::receive_event::<T>(event_id, event, signature))?)
        }

        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn publish_signature(origin, chain_id: ChainId, notice_id: NoticeId, signature: ChainSignature) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(check_failure::<T>(internal::notices::publish_signature(chain_id, notice_id, signature))?)
        }

        /// Offchain Worker entry point.
        fn offchain_worker(block_number: T::BlockNumber) {
            if let Err(e) = internal::events::fetch_events::<T>() {
                log!("offchain_worker error during fetch events: {:?}", e);
            }

            // XXX is this end times?
           if let Err(e) = internal::notices::process_notices::<T>(block_number) {
                log!("offchain_worker error during process notices: {:?}", e);
            }

            // XXX
            // todo: do this less often perhaps once a minute?
            // a really simple idea to do it once a minute on average is to just use probability
            // and only update the feed every now and then. It is a function of how many validators
            // are running.
            if let Err(e) = internal::oracle::process_prices::<T>(block_number) {
                log!("offchain_worker error during open price feed processing: {:?}", e);
            }
        }

        /// Execute a transaction request on behalf of a user
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
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

    /// Set the initial set of open price feed price reporters from the genesis config
    fn initialize_reporters(reporters: ReporterSet) {
        assert!(
            !reporters.is_empty(),
            "Open price feed price reporters must be set in the genesis config"
        );
        PriceReporters::put(reporters);
    }

    // ** API / View Functions ** //

    /// Get the liquidity for the given account.
    pub fn get_liquidity(account: ChainAccount) -> Result<AssetBalance, Reason> {
        Ok(core::get_liquidity::<T>(account)?.value)
    }

    /// Get the price for the given asset.
    pub fn get_price(ticker: Ticker) -> Result<AssetPrice, Reason> {
        Ok(core::get_price_by_ticker::<T>(ticker)?.value)
    }

    /// Get the rates for the given asset.
    pub fn get_rates(asset: ChainAsset) -> Result<(APR, APR), Reason> {
        Ok(core::get_rates::<T>(asset)?)
    }
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    /// Validate unsigned call to this module.
    ///
    /// By default unsigned transactions are disallowed, but implementing the validator
    /// here we make sure that some particular calls (the ones produced by offchain worker)
    /// are being whitelisted and marked as valid.
    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        match call {
            Call::set_miner(_miner) => ValidTransaction::with_tag_prefix("CashPallet::set_miner")
                .priority(1)
                .longevity(10)
                .propagate(true)
                .build(),
            Call::receive_event(event_id, event, signature) => {
                log!(
                    "validating receive_event({:?},{:?},{}))",
                    event_id,
                    event,
                    hex::encode(&signature)
                );

                ValidTransaction::with_tag_prefix("CashPallet::receive_event")
                    .priority(2)
                    .longevity(10)
                    .and_provides((event_id, signature))
                    .propagate(true)
                    .build()
            }
            Call::exec_trx_request(request, signature, nonce) => {
                let signer_res = signature.recover_account(
                    &internal::exec_trx_request::prepend_nonce(request, *nonce)[..],
                );

                log!("validating signer_res={:?}", signer_res);

                match (signer_res, nonce) {
                    (Err(_e), _) => InvalidTransaction::Call.into(),
                    (Ok(sender), 0) => {
                        ValidTransaction::with_tag_prefix("CashPallet::exec_trx_request")
                            .priority(4)
                            .longevity(10)
                            .and_provides((sender, 0))
                            .propagate(true)
                            .build()
                    }
                    (Ok(sender), _) => {
                        ValidTransaction::with_tag_prefix("CashPallet::exec_trx_request")
                            .priority(3)
                            .longevity(10)
                            .and_provides((sender, nonce))
                            .propagate(true)
                            .build()
                    }
                }
            }
            Call::post_price(_, sig) => ValidTransaction::with_tag_prefix("CashPallet::post_price")
                .priority(5)
                .longevity(10)
                .and_provides(sig)
                .propagate(true)
                .build(),
            Call::publish_signature(_, _, sig) => {
                ValidTransaction::with_tag_prefix("CashPallet::publish_signature")
                    .priority(5)
                    .longevity(10)
                    .and_provides(sig)
                    .propagate(true)
                    .build()
            }
            _ => InvalidTransaction::Call.into(),
        }
    }
}
