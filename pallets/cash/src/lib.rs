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
    Chain, ChainAccount, ChainAccountSignature, ChainAsset, ChainHash, ChainId, ChainSignature,
    ChainSignatureList, Ethereum,
};
use crate::events::{ChainLogEvent, ChainLogId, EventState};
use crate::notices::{Notice, NoticeId, NoticeState};
use crate::rates::{InterestRateModel, APR};
use crate::reason::Reason;
use crate::symbol::Ticker;
use crate::types::{
    AssetAmount, AssetBalance, AssetIndex, AssetInfo, AssetPrice, Bips, CashIndex, CashPrincipal,
    EncodedNotice, LiquidityFactor, Nonce, ReporterSet, SessionIndex, Timestamp, ValidatorKeys,
    ValidatorSig,
};
use codec::alloc::string::String;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, Parameter};
use frame_system::{
    ensure_none, ensure_root,
    offchain::{CreateSignedTransaction, SubmitTransaction},
};
use our_std::{if_std, str, vec::Vec};
use sp_core::crypto::AccountId32;
use sp_runtime::{
    offchain::{
        storage::StorageValueRef,
        storage_lock::{StorageLock, Time},
    },
    transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
    },
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
pub mod internal;
pub mod log;
pub mod notices;
pub mod oracle;
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

// OCW storage constants
pub const OCW_STORAGE_LOCK_ETHEREUM_EVENTS: &[u8; 34] = b"cash::storage_lock_ethereum_events";
pub const OCW_LATEST_CACHED_ETHEREUM_BLOCK: &[u8; 34] = b"cash::latest_cached_ethereum_block";
pub const OCW_LATEST_PRICE_FEED_TIMESTAMP: &[u8; 33] = b"cash::latest_price_feed_timestamp";
pub const OCW_LATEST_PRICE_FEED_POLL_BLOCK_NUMBER: &[u8; 41] =
    b"cash::latest_price_feed_poll_block_number";
pub const OCW_STORAGE_LOCK_OPEN_PRICE_FEED: &[u8; 34] = b"cash::storage_lock_open_price_feed";

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

    type TimeConverter: Convert<<Self as pallet_timestamp::Config>::Moment, Timestamp>;
}

decl_storage! {
    trait Store for Module<T: Config> as Cash {
        /// The timestamp of the previous block (or defaults to timestamp of the genesis block).
        LastBlockTimestamp get(fn last_block_timestamp) config(): Timestamp;

        NextSessionIndex get(fn get_next_session_index): SessionIndex;

        /// The upcoming set of allowed validators, and their associated keys (or none).
        NextValidators get(fn next_validators) : map hasher(blake2_128_concat) SubstrateId => Option<ValidatorKeys>;

        /// The current set of allowed validators, and their associated keys.
        Validators get(fn validators) : map hasher(blake2_128_concat) SubstrateId => Option<ValidatorKeys>;

        /// An index to track interest earned by CASH holders and owed by CASH borrowers.
        GlobalCashIndex get(fn cash_index): CashIndex;

        /// The upcoming base rate change for CASH and when, if any.
        CashYieldNext get(fn cash_yield_next): Option<(APR, Timestamp)>;

        /// The current APR on CASH held, and the base rate paid by borrowers.
        CashYield get(fn cash_yield): APR;

        /// The liquidation incentive on seized collateral (e.g. 8% = 800 bips).
        GlobalLiquidationIncentive get(fn liquidation_incentive): Bips;

        /// The fraction of borrower interest that is paid to the protocol (e.g. 1/10th = 1000 bips).
        Spreads get(fn spread): map hasher(blake2_128_concat) ChainAsset => Bips;

        /// The mapping of indices to track interest owed by asset borrowers, by asset.
        BorrowIndices get(fn borrow_index): map hasher(blake2_128_concat) ChainAsset => AssetIndex;

        /// The mapping of indices to track interest earned by asset suppliers, by asset.
        SupplyIndices get(fn supply_index): map hasher(blake2_128_concat) ChainAsset => AssetIndex;

        /// The total CASH principal held per chain.
        ChainCashPrincipals get(fn chain_cash_principal): map hasher(blake2_128_concat) ChainId => CashPrincipal;

        /// The total CASH principal in existence.
        TotalCashPrincipal get(fn total_cash_principal): CashPrincipal;

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
        Prices get(fn price): map hasher(blake2_128_concat) Ticker => AssetPrice;

        /// Mapping of assets to the last time their price was updated.
        PriceTimes get(fn price_time): map hasher(blake2_128_concat) Ticker => Timestamp;

        /// Ethereum addresses of open oracle price reporters.
        PriceReporters get(fn reporters): ReporterSet; // XXX if > 1, how are we combining?

        /// The asset metadata for each supported asset, which will also be synced with the starports.
        SupportedAssets get(fn asset): map hasher(blake2_128_concat) ChainAsset => Option<AssetInfo>;

        /// Mapping of strings to tickers (valid tickers indexed by ticker string).
        Tickers get(fn ticker): map hasher(blake2_128_concat) String => Option<Ticker>;
    }
    add_extra_genesis {
        config(assets): Vec<AssetInfo>;
        config(initial_yield): Option<(APR, Timestamp)>;
        config(reporters): ReporterSet;
        config(validators): Vec<ValidatorKeys>;
        build(|config| {
            Module::<T>::initialize_assets(config.assets.clone());
            Module::<T>::initialize_reporters(config.reporters.clone());
            Module::<T>::initialize_validators(config.validators.clone());
            Module::<T>::initialize_yield(config.initial_yield.clone());
        })
    }
}

/* ::EVENTS:: */

decl_event!(
    pub enum Event {
        /// XXX -- For testing
        GoldieLocks(ChainAsset, ChainAccount, AssetAmount),

        /// An Ethereum event was successfully processed. [event_id]
        ProcessedChainEvent(ChainLogId),

        /// An Ethereum event failed during processing. [event_id, reason]
        FailedProcessingChainEvent(ChainLogId, Reason),

        /// Signed notice. [chain_id, notice_id, message, signatures]
        SignedNotice(ChainId, NoticeId, EncodedNotice, ChainSignatureList),

        /// When a new notice is generated
        Notice(NoticeId, Notice, EncodedNotice),

        /// Executed Governance Action
        ExecutedGovernance(Vec<(Vec<u8>, bool)>),
    }
);

/* ::ERRORS:: */

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Error returned when fetching starport info
        HttpFetchingError,

        /// An error related to the oracle
        OpenOracleError,

        /// Open oracle cannot parse the signature
        OpenOracleErrorInvalidSignature,

        /// Open oracle cannot parse the signature
        OpenOracleErrorInvalidReporter,

        /// Open oracle cannot parse the message
        OpenOracleErrorInvalidMessage,

        /// Open oracle cannot parse the ticker
        OpenOracleErrorInvalidTicker,

        /// Open oracle cannot update price due to stale price
        OpenOracleErrorStalePrice,

        /// Open oracle cannot fetch price due to an http error
        OpenOracleHttpFetchError,

        /// Open oracle cannot fetch price due to an http error
        OpenOracleBadUrl,

        /// Open oracle cannot deserialize the messages and/or signatures from eth encoded hex to binary
        OpenOracleApiResponseHexError,

        /// Open oracle offchain worker made an attempt to update the price but failed in the extrinsic
        OpenOraclePostPriceExtrinsicError,

        /// An error related to the chain_spec file contents
        GenesisConfigError,

        /// Invalid parameters to a provided interest rate model
        InterestRateModelInvalidParameters,

        /// Asset is not supported
        AssetNotSupported,

        /// No interest rate model is provided for the given asset
        InterestRateModelNotSet,

        /// Could not get the borrow rate on the asset but there was a model to try
        InterestRateModelGetBorrowRateFailed,

        /// Could not get the utilization ratio
        InterestRateModelGetUtilizationFailed,

        /// Signature recovery errror
        InvalidSignature,

        /// Error signing notice in off-chain worker
        ProcessNoticeFailure,

        /// Failed to publish signatures
        PublishSignatureFailure,

        /// Error processing trx request
        TrxRequestError,

        /// Error when processing chain event
        ErrorProcessingChainEvent,

        /// Error when setting a future yield rate
        SetYieldNextFailure
    }
}

macro_rules! r#cash_err {
    ($expr:expr, $new_err:expr) => {
        match $expr {
            core::result::Result::Ok(val) => Ok(val),
            core::result::Result::Err(err) => {
                if_std! {
                    println!("Error {:#?}", err)
                }
                log!("Error {:#?}", err);
                Err($new_err)
            }
        }
    };
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
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        fn on_initialize(block: T::BlockNumber) -> frame_support::weights::Weight {
            Self::on_initialize_internal(block)
        }

        /// Set the price using the open price feed.
        #[weight = 0]
        pub fn post_price(origin, payload: Vec<u8>, signature: Vec<u8>) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Self::post_price_internal(payload, signature)?;
            Ok(())
        }


        /// Set the price using the open price feed.
        #[weight = 0]
        pub fn set_liquidity_factor(origin, asset: ChainAsset, factor: LiquidityFactor) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Self::set_liquidity_factor_internal(asset, factor)?;
            Ok(())
        }

        /// Update the interest rate model for a given asset. This is only called by governance
        #[weight = 0]
        pub fn set_rate_model(origin, asset: ChainAsset, model: InterestRateModel) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Self::set_rate_model_internal(asset, model)?;
            Ok(())
        }

        /// Set the cash yield rate at some point in the future
        #[weight = 0]
        pub fn set_yield_next(origin, next_apr: APR, next_apr_start: Timestamp) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            cash_err!(
                crate::internal::set_yield_next::set_yield_next::<T>(next_apr, next_apr_start),
                <Error<T>>::SetYieldNextFailure
            )?;

            Ok(())
        }

        #[weight = 0]
        pub fn change_authorities(origin, keys: Vec<(AccountId32, ValidatorKeys)>) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Self::change_authorities_internal(keys);
            Ok(())
        }

        /// Execute a transaction request on behalf of a user
        #[weight = 1]
        pub fn exec_trx_request(origin, request: Vec<u8>, signature: ChainAccountSignature, nonce: Nonce) -> dispatch::DispatchResult {
            ensure_none(origin)?;

            cash_err!(core::exec_trx_request_internal::<T>(request, signature, nonce), Error::<T>::TrxRequestError)?;
            Ok(())
        }

        // TODO: Do we need to sign the event id, too?
        #[weight = 1] // XXX how are we doing weights?
        pub fn process_chain_event(origin, event_id: ChainLogId, event: ChainLogEvent, signature: ValidatorSig) -> dispatch::DispatchResult { // XXX sig
            log!("process_chain_event(origin,event_id,event,sig): {:?} {:?} {}", event_id, &event, hex::encode(&signature));
            ensure_none(origin)?;

            cash_err!(core::process_chain_event_internal::<T>(event_id, event, signature), Error::<T>::ErrorProcessingChainEvent)?;
            Ok(())
        }

        #[weight = 0] // XXX
        pub fn publish_signature(origin, chain_id: ChainId, notice_id: NoticeId, signature: ChainSignature) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            cash_err!(core::publish_signature_internal(chain_id, notice_id, signature), Error::<T>::PublishSignatureFailure)?;

            Ok(())
        }

        /// Offchain Worker entry point.
        fn offchain_worker(block_number: T::BlockNumber) {
            let result = Self::fetch_events_with_lock();
            if let Err(e) = result {
                log!("offchain_worker error during fetch events: {:?}", e);
            }

            // TODO: What to do with res?
            let _res = cash_err!(core::process_notices::<T>(block_number), Error::<T>::ProcessNoticeFailure);

            // todo: do this less often perhaps once a minute?
            // a really simple idea to do it once a minute on average is to just use probability
            // and only update the feed every now and then. It is a function of how many validators
            // are running.
            let result = Self::process_open_price_feed(block_number);
            if let Err(e) = result {
                log!("offchain_worker error during open price feed processing: {:?}", e);
            }
        }
    }
}

/// Reading error messages inside `decl_module!` can be difficult, so we move them here.
impl<T: Config> Module<T> {
    /// Called by substrate on block initialization. Note, this can fail under various scenarios
    /// but the function signature does not allow us to express that to the substrate system.
    /// This is our final stand
    fn on_initialize_internal(block: T::BlockNumber) -> frame_support::weights::Weight {
        match crate::core::on_initialize_core::<T>() {
            Ok(weight) => weight,
            Err(err) => {
                // todo: something is going very wrong here..... we need to take some very serious
                // action like invalidating the block if possible.
                error!("Could not initialize block! {:#?} {:#?}", block, err);
                0
            }
        }
    }

    /// Update the liquidity factor for an asset.
    fn set_liquidity_factor_internal(
        asset: ChainAsset,
        factor: LiquidityFactor,
    ) -> Result<(), Error<T>> {
        let asset_info = SupportedAssets::get(asset).ok_or(<Error<T>>::AssetNotSupported)?;
        SupportedAssets::insert(
            &asset,
            AssetInfo {
                liquidity_factor: factor,
                ..asset_info
            },
        );
        Ok(())
    }

    /// Update the interest rate model for an asset.
    fn set_rate_model_internal(
        asset: ChainAsset,
        model: InterestRateModel,
    ) -> Result<(), Error<T>> {
        let asset_info = SupportedAssets::get(asset).ok_or(<Error<T>>::AssetNotSupported)?;
        // XXX err pattern?
        cash_err!(
            model.check_parameters(),
            <Error<T>>::InterestRateModelInvalidParameters
        )?;
        SupportedAssets::insert(
            &asset,
            AssetInfo {
                rate_model: model,
                ..asset_info
            },
        );
        Ok(())
    }

    fn change_authorities_internal(keys: Vec<(AccountId32, ValidatorKeys)>) {
        for (id, _chain_keys) in <NextValidators>::iter() {
            <NextValidators>::take(id);
        }

        for (id, chain_keys) in &keys {
            <NextValidators>::take(id);
            <NextValidators>::insert(&id, chain_keys);
        }
    }

    /// Body of the post_price extrinsic
    /// * checks signature of message
    /// * parses message
    /// * updates storage
    ///
    /// Gets us out of the decl_module! macro and helps with static code analysis to have the
    /// body of this function here.
    fn post_price_internal(payload: Vec<u8>, signature: Vec<u8>) -> Result<(), Error<T>> {
        // check signature
        let parsed_sig: <Ethereum as Chain>::Signature = cash_err!(
            compound_crypto::eth_signature_from_bytes(&signature),
            <Error<T>>::OpenOracleErrorInvalidSignature
        )?;
        // note that this is actually a double-hash situation but that is expected behavior
        // the hashed message is hashed again in the eth convention inside eth_recover
        let hashed = compound_crypto::keccak(&payload);
        let recovered = cash_err!(
            compound_crypto::eth_recover(&hashed, &parsed_sig, true),
            <Error<T>>::OpenOracleErrorInvalidSignature
        )?;
        if !PriceReporters::get().contains(recovered) {
            return Err(<Error<T>>::OpenOracleErrorInvalidReporter);
        }

        // parse message and check it
        let parsed = cash_err!(
            oracle::parse_message(&payload),
            <Error<T>>::OpenOracleErrorInvalidMessage
        )?;

        let ticker: Ticker = cash_err!(
            str::FromStr::from_str(&parsed.key),
            <Error<T>>::OpenOracleErrorInvalidTicker
        )?;

        log!(
            "Parsed price from open price feed: {:?} is worth {:?}",
            parsed.key,
            (parsed.value as f64) / 1000000.0
        );
        // todo: more sanity checking on the value
        // note - the API for GET does not return an Option but if the value is not present it
        // returns Default::default(). Thus, we explicitly check if the key for the ticker is supported
        // or not.

        if PriceTimes::contains_key(&ticker) {
            // it has been updated at some point, make sure we are updating to a more recent price
            let last_updated = PriceTimes::get(&ticker);
            if parsed.timestamp <= last_updated {
                return Err(<Error<T>>::OpenOracleErrorStalePrice);
            }
        }

        // WARNING begin storage - all checks must happen above

        Prices::insert(&ticker, parsed.value as u128);
        PriceTimes::insert(&ticker, parsed.timestamp as u128);

        // todo: update storage
        Ok(())
    }

    // XXX expose price(String) fn

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

    /// Set the initial cash yield, if provided
    fn initialize_yield(initial_yield_config: Option<(APR, Timestamp)>) {
        if let Some((initial_yield, initial_yield_start)) = initial_yield_config {
            CashYield::put(initial_yield);
            LastBlockTimestamp::put(initial_yield_start);
        }
    }

    /// Procedure for offchain worker to processes messages coming out of the open price feed
    pub fn process_open_price_feed(block_number: T::BlockNumber) -> Result<(), Error<T>> {
        let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_OPEN_PRICE_FEED);
        if lock.try_lock().is_err() {
            // working in another thread, no big deal
            return Ok(());
        }

        // get the URL to poll, just return if there is no URL set up
        let url = runtime_interfaces::validator_config_interface::get_opf_url().unwrap_or(vec![]);
        if url.len() == 0 {
            return Ok(());
        }

        // check to see if it is time to poll or not
        let latest_price_feed_poll_block_number_storage =
            StorageValueRef::persistent(OCW_LATEST_PRICE_FEED_POLL_BLOCK_NUMBER);
        if let Some(Some(latest_poll_block_number)) =
            latest_price_feed_poll_block_number_storage.get::<T::BlockNumber>()
        {
            let poll_interval_blocks = <T as frame_system::Config>::BlockNumber::from(
                params::OCW_OPEN_ORACLE_POLL_INTERVAL_BLOCKS,
            );
            if block_number - latest_poll_block_number < poll_interval_blocks {
                return Ok(());
            }
        }
        let url = cash_err!(String::from_utf8(url), <Error<T>>::OpenOracleBadUrl)?;

        // poll
        let api_response = cash_err!(
            crate::oracle::open_price_feed_request(&url),
            <Error<T>>::OpenOracleHttpFetchError
        )?;

        let messages_and_signatures_and_timestamp = cash_err!(
            api_response.to_message_signature_pairs(),
            <Error<T>>::OpenOracleApiResponseHexError
        )?;
        let (messages_and_signatures, timestamp) = messages_and_signatures_and_timestamp;

        // Check to see if Coinbase api prices were updated or not
        let latest_price_feed_timestamp_storage =
            StorageValueRef::persistent(OCW_LATEST_PRICE_FEED_TIMESTAMP);
        if let Some(Some(latest_price_feed_timestamp)) =
            latest_price_feed_timestamp_storage.get::<String>()
        {
            if latest_price_feed_timestamp == timestamp {
                log!(
                    "Open oracle prices for timestamp {:?} has been already posted",
                    timestamp
                );
                return Ok(());
            }
        }

        for (msg, sig) in messages_and_signatures {
            // adding some debug info in here, this will become very chatty
            let call = <Call<T>>::post_price(msg, sig);
            let _ = cash_err!(
                SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()),
                <Error<T>>::OpenOraclePostPriceExtrinsicError
            );
            // note - there is a log message in cash_err if this extrinsic fails but we should
            // still try to update the other prices even if one extrinsic fails, thus the result
            // is ignored and we continue in this loop
        }
        latest_price_feed_poll_block_number_storage.set(&block_number);
        latest_price_feed_timestamp_storage.set(&timestamp);
        Ok(())
    }

    // XXX disambiguate whats for ethereum vs not
    fn fetch_events_with_lock() -> Result<(), Error<T>> {
        let s_info = StorageValueRef::persistent(OCW_LATEST_CACHED_ETHEREUM_BLOCK);

        let from_block: String = if let Some(Some(cached_block_num)) = s_info.get::<u64>() {
            // Ethereum block number has been cached, fetch events starting from the next after cached block
            log!("Last cached block number: {:?}", cached_block_num);

            events::encode_block_hex(cached_block_num + 1)
        } else {
            // Validator's cache is empty, fetch events from the earliest block with pending events
            log!("Block number has not been cached yet");
            // XXX TODO: Add back?
            // let block_numbers: Vec<u32> = EthEventQueue::iter()
            //     .filter_map(|((block_number, _log_index), status)| {
            //         if match status {
            //             EventStatus::<Ethereum>::Pending { .. } => true,
            //             _ => false,
            //         } {
            //             Some(block_number)
            //         } else {
            //             None
            //         }
            //     })
            //     .collect();
            // let pending_events_block = block_numbers.iter().min();
            // if pending_events_block.is_some() {
            //     let events_block: u32 = *pending_events_block.unwrap();
            //     from_block = format!("{:#X}", events_block);
            // } else {
            //}
            String::from("earliest")
        };

        log!("Fetching events starting from block {:?}", from_block);

        let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_ETHEREUM_EVENTS);

        if let Ok(_guard) = lock.try_lock() {
            match events::fetch_events(from_block) {
                Ok(event_info) => {
                    log!("Result: {:?}", event_info);

                    // TODO: The failability here is bad, since we don't want to re-process events
                    // We need to make sure this is fully idempotent

                    // Send extrinsics for all events
                    cash_err!(
                        core::process_events_internal::<T>(event_info.events),
                        Error::<T>::ErrorProcessingChainEvent
                    )?;

                    // Save latest block in ocw storage
                    s_info.set(&event_info.latest_eth_block);
                }
                Err(err) => {
                    log!("Error while fetching events: {:?}", err);
                    return Err(Error::<T>::HttpFetchingError);
                }
            }
        }
        Ok(())
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
            Call::process_chain_event(_event_id, _event, signature) => {
                ValidTransaction::with_tag_prefix("CashPallet")
                    .longevity(10)
                    .and_provides(signature)
                    .propagate(true)
                    .build()
            }
            Call::exec_trx_request(request, signature, nonce) => {
                let signer_res =
                    signature.recover_account(&core::prepend_nonce(request, *nonce)[..]);

                log!("signer_res={:?}", signer_res);

                match (signer_res, nonce) {
                    (Err(_e), _) => InvalidTransaction::Call.into(),
                    (Ok(sender), 0) => ValidTransaction::with_tag_prefix("CashPallet")
                        .longevity(10)
                        .and_provides((sender, 0))
                        .propagate(true)
                        .build(),
                    (Ok(sender), _) => ValidTransaction::with_tag_prefix("CashPallet")
                        .longevity(10)
                        .and_provides((sender, nonce))
                        .propagate(true)
                        .build(),
                }
            }
            Call::post_price(_, sig) => ValidTransaction::with_tag_prefix("CashPallet")
                .longevity(10)
                .and_provides(sig)
                .propagate(true)
                .build(),
            Call::publish_signature(_, _, sig) => ValidTransaction::with_tag_prefix("CashPallet")
                .longevity(10)
                .and_provides(sig)
                .propagate(true)
                .build(),
            _ => InvalidTransaction::Call.into(),
        }
    }
}
