#![allow(incomplete_features)]
#![feature(array_methods)]
#![feature(associated_type_defaults)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(const_panic)]
#![feature(str_split_once)]

#[macro_use]
extern crate alloc;
extern crate ethereum_client;
extern crate trx_request;

use crate::chains::{
    Chain, ChainAccount, ChainAsset, ChainId, ChainSignature, ChainSignatureList, Ethereum,
};
use crate::notices::{Notice, NoticeId, NoticeStatus};
use crate::rates::{InterestRateModel, APR};
use crate::reason::Reason;
use crate::symbol::Symbol;
use crate::types::{
    AssetAmount, AssetBalance, AssetIndex, AssetPrice, Bips, CashIndex, CashPrincipal, ChainKeys,
    ConfigAsset, ConfigSetString, EncodedNotice, EthAddress, EventStatus, Nonce, Quantity,
    ReporterSet, SessionIndex, SignedPayload, Timestamp, ValidatorSet, ValidatorSig,
};
use codec::{alloc::string::String, Decode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{
    ensure_none, ensure_root,
    offchain::{CreateSignedTransaction, SubmitTransaction},
};
use our_std::{
    convert::{TryFrom, TryInto},
    str,
    vec::Vec,
};
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

use pallet_session;

pub mod chains;
pub mod core;
pub mod events;
pub mod log;
pub mod notices;
pub mod oracle;
pub mod params;
pub mod rates;
pub mod reason;
pub mod symbol;
pub mod trx_req;
pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod testdata;

// OCW storage constants
pub const OCW_STORAGE_LOCK_ETHEREUM_EVENTS: &[u8; 34] = b"cash::storage_lock_ethereum_events";
pub const OCW_LATEST_CACHED_ETHEREUM_BLOCK: &[u8; 34] = b"cash::latest_cached_ethereum_block";
pub const OCW_LATEST_PRICE_FEED_TIMESTAMP: &[u8; 33] = b"cash::latest_price_feed_timestamp";
pub const OCW_LATEST_PRICE_FEED_POLL_BLOCK_NUMBER: &[u8; 41] =
    b"cash::latest_price_feed_poll_block_number";
pub const OCW_STORAGE_LOCK_OPEN_PRICE_FEED: &[u8; 34] = b"cash::storage_lock_open_price_feed";

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;

    /// The overarching dispatch call type.
    type Call: From<Call<Self>>;
}

decl_storage! {
    trait Store for Module<T: Config> as Cash {
        /// The timestamp of the previous block (or defaults to timestamp of the genesis block).
        LastBlockTimestamp get(fn last_block_timestamp) config(): Timestamp;

        NextSessionIndex get(fn get_next_session_index): SessionIndex;

        /// The upcoming set of allowed validators, and their associated keys (or none).
        NextValidators get(fn next_validators) : map hasher(blake2_128_concat) AccountId32 => Option<ChainKeys>;

        /// The current set of allowed validators, and their associated keys.
        Validators get(fn validators) : map hasher(blake2_128_concat) AccountId32 => Option<ChainKeys>;

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

        /// The mapping of asset balances, by chain and account.
        AssetBalances get(fn asset_balance): double_map hasher(blake2_128_concat) ChainAsset, hasher(blake2_128_concat) ChainAccount => AssetBalance;

        // XXX break into separate storage
        //  liquidity factor, supply cap, etc.
        /// The asset metadata to be synced with the starports.
        AssetMetadata get (fn asset_metadata): map hasher(blake2_128_concat) ChainAsset => ();

        /// Mapping of assets to symbols, which determines if an asset is supported.
        AssetSymbols get(fn asset_symbol): map hasher(blake2_128_concat) ChainAsset => Option<Symbol>;

        /// The mapping of (status of) events witnessed on Ethereum, by event id.
        EthEventQueue get(fn eth_event_queue): map hasher(blake2_128_concat) chains::eth::EventId => Option<EventStatus<Ethereum>>;

        /// The mapping of (status of) notices to be signed for Ethereum, by notice id.
        NoticeQueue get(fn notice_queue): map hasher(blake2_128_concat) NoticeId => Option<NoticeStatus>;

        /// The last used nonce for each account, initialized at zero.
        Nonces get(fn nonces): map hasher(blake2_128_concat) ChainAccount => Nonce;

        /// The mapping of interest rate models, by asset.
        RateModels get(fn model): map hasher(blake2_128_concat) ChainAsset => InterestRateModel;

        /// Mapping of latest prices for each asset symbol.
        Prices get(fn price): map hasher(blake2_128_concat) Symbol => AssetPrice;

        /// Mapping of assets to the last time their price was updated.
        PriceTimes get(fn price_time): map hasher(blake2_128_concat) Symbol => Timestamp;

        /// Ethereum addresses of open oracle price reporters.
        PriceReporters get(fn reporters): ReporterSet; // XXX if > 1, how are we combining?

        /// Mapping of strings to symbols (valid asset symbols indexed by ticker string).
        Symbols get(fn symbol): map hasher(blake2_128_concat) String => Option<Symbol>;
    }
    add_extra_genesis {
        config(validator_ids): Vec<[u8;32]>;
        config(validator_keys): Vec<(String,)>;
        config(reporters): ConfigSetString;
        config(assets): Vec<ConfigAsset>;
        build(|config| {
            Module::<T>::initialize_assets(config.assets.clone());
            Module::<T>::initialize_reporters(config.reporters.clone());
            Module::<T>::initialize_validators(config.validator_ids.clone(), config.validator_keys.clone());
        })
    }
}

decl_event!(
    pub enum Event {
        /// XXX -- For testing
        GoldieLocks(ChainAsset, ChainAccount, AssetAmount),

        /// An Ethereum event was successfully processed. [payload]
        ProcessedEthEvent(SignedPayload),

        /// An Ethereum event failed during processing. [payload, reason]
        FailedProcessingEthEvent(SignedPayload, Reason),

        /// Signed notice. [chain_id, notice_id, message, signatures]
        SignedNotice(ChainId, NoticeId, EncodedNotice, ChainSignatureList),

        /// When a new notice is generated
        Notice(NoticeId, Notice, EncodedNotice),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Error returned when fetching starport info
        HttpFetchingError,

        /// Error when processing `Lock` event while sending `process_eth_event` extrinsic
        ProcessLockEventError,

        /// Error sending `process_eth_event` extrinsic
        OffchainUnsignedLockTxError,

        /// Error decoding payload
        SignedPayloadError,

        /// Public key doesn't belong to known validator
        UnknownValidator,

        /// Validator has already signed and submitted this payload
        AlreadySigned,

        /// Fetched Ethereum event type is not known
        UnknownEthEventType, // XXX needed per-chain?

        /// Error decoding Ethereum event
        DecodeEthereumEventError,

        /// An error related to the oracle
        OpenOracleError,

        /// Open oracle cannot parse the signature
        OpenOracleErrorInvalidSignature,

        /// Open oracle cannot parse the signature
        OpenOracleErrorInvalidReporter,

        /// Open oracle cannot parse the message
        OpenOracleErrorInvalidMessage,

        /// Open oracle cannot update price due to stale price
        OpenOracleErrorStalePrice,

        /// Open oracle cannot fetch price due to an http error
        OpenOracleHttpFetchError,

        /// Open oracle cannot deserialize the messages and/or signatures from eth encoded hex to binary
        OpenOracleApiResponseHexError,

        /// Open oracle offchain worker made an attempt to update the price but failed in the extrinsic
        OpenOraclePostPriceExtrinsicError,

        /// An unsupported symbol was provided to the oracle.
        OpenOracleErrorInvalidSymbol,

        /// An error related to the chain_spec file contents
        GenesisConfigError,

        /// Trx request parsing error
        TrxRequestParseError,

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

        /// Notice issue
        InvalidNoticeParams,

        /// Signature recovery errror
        InvalidSignature,

        /// Error signing notice in off-chain worker
        ProcessNoticeFailure,

        /// Failed to publish signatures
        PublishSignatureFailure,

        /// Invalid events block number
        EventsBlockNumberError,
        /// Error processing trx request
        TrxRequestError,
    }
}

macro_rules! r#cash_err {
    ($expr:expr, $new_err:expr) => {
        match $expr {
            core::result::Result::Ok(val) => Ok(val),
            core::result::Result::Err(err) => {
                log!("Error {:#?}", err);
                Err($new_err)
            }
        }
    };
}

impl<T: Config> pallet_session::SessionManager<AccountId32> for Module<T> {
    // return validator set to use in the next session (babe and grandpa also stage new auths associated w these accountIds)
    fn new_session(session_index: SessionIndex) -> Option<Vec<AccountId32>> {
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
                <NextValidators>::take(&kv.0);
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

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        /// Set the price using the open price feed.
        #[weight = 0]
        pub fn post_price(origin, payload: Vec<u8>, signature: Vec<u8>) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Self::post_price_internal(payload, signature)?;
            Ok(())
        }

        /// Update the interest rate model for a given asset. This is only called by governance
        #[weight = 0]
        pub fn update_interest_rate_model(origin, asset: ChainAsset, model: InterestRateModel) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Self::update_interest_rate_model_internal(asset, model)?;
            Ok(())
        }

        // TODO
        #[weight = 1]
        pub fn exec_trx_request(origin, request: Vec<u8>, signature: ChainSignature) -> dispatch::DispatchResult {
            ensure_none(origin)?;

            // // TODO: Add more error information here
            let request_str: &str = str::from_utf8(&request[..]).map_err(|_| <Error<T>>::TrxRequestParseError)?;

            // // TODO: Add more error information here
            let sender = cash_err!(signature.recover(&request), <Error<T>>::InvalidSignature)?; // XXX error from reason?
            let trx_request = cash_err!(trx_request::parse_request(request_str), <Error<T>>::TrxRequestParseError)?;

            match trx_request {
                trx_request::TrxRequest::Extract(amount, asset, account) => {
                    let chain_asset: ChainAsset = asset.into();
                    let symbol = core::symbol::<T>(chain_asset).ok_or(<Error<T>>::AssetNotSupported)?; // TODO: Correct error? cash_err?
                    let quantity = Quantity(symbol, amount.into());
                    match core::extract_internal::<T>(chain_asset, sender, account.into(), quantity) {
                        Ok(()) => Ok(()),
                        Err(err) => {
                            log!("TrxExtract Error: {:?}", err);
                            Err(Error::<T>::TrxRequestError)?
                        }
                    }
                }
            }
        }

        #[weight = 1] // XXX how are we doing weights?
        pub fn process_eth_event(origin, payload: SignedPayload, signature: ValidatorSig) -> dispatch::DispatchResult { // XXX sig
            log!("process_eth_event(origin,payload,sig): {} {}", hex::encode(&payload), hex::encode(&signature));
            ensure_none(origin)?;

            // XXX do we want to store/check hash to allow replaying?
            // TODO: use more generic function?
            let signer: crate::types::ValidatorKey = cash_err!(
                compound_crypto::eth_recover(&payload[..], &signature, false),  // XXX why is this using eth for validator sig though?
                Error::<T>::InvalidSignature)?;

            let validators: Vec<EthAddress> = <Validators>::iter().map(|v| v.1.eth_address).collect();

            if !validators.contains(&signer) {
                log!("Signer of a payload is not a known validator {:?}, validators are {:?}", signer, validators);
                return Err(Error::<T>::UnknownValidator)?
            }

            let event = chains::eth::Event::decode(&mut payload.as_slice()).map_err(|_| <Error<T>>::DecodeEthereumEventError)?; // XXX
            let status = <EthEventQueue>::get(event.id).unwrap_or(EventStatus::<Ethereum>::Pending { signers: vec![] }); // XXX
            match status {
                EventStatus::<Ethereum>::Pending { signers } => {
                    // XXX sets?
                    if signers.contains(&signer) {
                        log!("Validator has already signed this payload {:?}", signer);
                        return Err(Error::<T>::AlreadySigned)?
                    }

                    // Add new validator to the signers
                    let mut signers_new = signers.clone();
                    signers_new.push(signer.clone()); // XXX unique add to set?

                    if core::passes_validation_threshold(&signers_new, &validators) {
                        match core::apply_eth_event_internal::<T>(event) {
                            Ok(()) => {
                                EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Done);
                                Self::deposit_event(Event::ProcessedEthEvent(payload));
                                Ok(())
                            }

                            Err(reason) => {
                                EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Failed { hash: Ethereum::hash_bytes(payload.as_slice()), reason: reason });
                                Self::deposit_event(Event::FailedProcessingEthEvent(payload, reason));
                                Ok(())
                            }
                        }
                    } else {
                        EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Pending { signers: signers_new });
                        Ok(())
                    }
                }

                // Retry logic to allow retrying Failures (which should never happen, but theoretically can)
                EventStatus::<Ethereum>::Failed { hash, .. } => {
                    // XXX require(compute_hash(payload) == hash, "event data differs from failure");
                    match core::apply_eth_event_internal::<T>(event) {
                        Ok(_) => {
                            EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Done);
                            Self::deposit_event(Event::ProcessedEthEvent(payload));
                            Ok(())
                        }

                        Err(new_reason) => {
                            EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Failed { hash, reason: new_reason});
                            Self::deposit_event(Event::FailedProcessingEthEvent(payload, new_reason));
                            Ok(())
                        }
                    }
                }

                EventStatus::<Ethereum>::Done => {
                    // TODO: Eventually we should be deleting here (based on monotonic ids and signing order)
                    Ok(())
                }
            }
        }

        #[weight = 0] // XXX
        pub fn publish_signature(origin, notice_id: NoticeId, signature: ChainSignature) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            cash_err!(core::publish_signature_internal(notice_id, signature), Error::<T>::PublishSignatureFailure)?;

            Ok(())
        }

        #[weight = 0] // XXX
        pub fn change_authorities(origin, keys: Vec<(AccountId32, ChainKeys)>) -> dispatch::DispatchResult {
            // TODO: assert root only

            for (id, _chain_keys) in <NextValidators>::iter() {
                <NextValidators>::take(id);
            }

            for (id, chain_keys) in &keys {
                <NextValidators>::take(id);
                <NextValidators>::insert(&id, chain_keys);
            }

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
    /// Check to make sure an asset is supported. Only returns Err when the asset is not supported
    /// by compound chain.
    fn check_asset_supported(asset: &ChainAsset) -> Result<(), Error<T>> {
        if !AssetSymbols::contains_key(&asset) {
            return Err(<Error<T>>::AssetNotSupported);
        }

        Ok(())
    }

    /// Update the interest rate model
    fn update_interest_rate_model_internal(
        asset: ChainAsset,
        model: InterestRateModel,
    ) -> Result<(), Error<T>> {
        Self::check_asset_supported(&asset)?;
        cash_err!(
            model.check_parameters(),
            <Error<T>>::InterestRateModelInvalidParameters
        )?;
        RateModels::insert(&asset, model);
        Ok(())
    }

    /// Get the borrow rate for the given asset
    pub fn get_borrow_rate(asset: &ChainAsset) -> Result<APR, Error<T>> {
        if !RateModels::contains_key(asset) {
            // XXX we should just make this part of the model getter
            return Err(<Error<T>>::InterestRateModelNotSet);
        }
        let model = RateModels::get(asset);
        let utilization = Self::get_utilization(asset)?;
        let rate = cash_err!(
            model.get_borrow_rate(utilization, 0),
            <Error<T>>::InterestRateModelGetBorrowRateFailed
        )?;
        Ok(rate)
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
        let reporters = PriceReporters::get();
        if !reporters
            .iter()
            .any(|e| e.as_slice() == recovered.as_slice())
        {
            return Err(<Error<T>>::OpenOracleErrorInvalidReporter);
        }

        // parse message and check it
        let parsed = cash_err!(
            oracle::parse_message(&payload),
            <Error<T>>::OpenOracleErrorInvalidMessage
        )?;

        let symbol = cash_err!(
            Symbols::get(parsed.key.clone()).ok_or(<Error<T>>::OpenOracleErrorInvalidSymbol),
            <Error<T>>::OpenOracleErrorInvalidSymbol
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

        if PriceTimes::contains_key(&symbol) {
            // it has been updated at some point, make sure we are updating to a more recent price
            let last_updated = PriceTimes::get(&symbol);
            if parsed.timestamp <= last_updated {
                return Err(<Error<T>>::OpenOracleErrorStalePrice);
            }
        }

        // WARNING begin storage - all checks must happen above

        Prices::insert(&symbol, parsed.value as u128);
        PriceTimes::insert(&symbol, parsed.timestamp as u128);

        // todo: update storage
        Ok(())
    }

    /// Get the utilization in the provided asset.
    pub fn get_utilization(asset: &ChainAsset) -> Result<crate::rates::Utilization, Error<T>> {
        Self::check_asset_supported(asset)?;
        let total_supply = TotalSupplyAssets::get(asset);
        let total_borrow = TotalBorrowAssets::get(asset);
        let utilization = cash_err!(
            rates::get_utilization(total_supply, total_borrow),
            <Error<T>>::InterestRateModelGetUtilizationFailed
        )?;
        Ok(utilization)
    }

    /// Convert from the chain spec file format hex encoded strings into the local storage format, binary blob
    fn hex_string_vec_to_binary_vec<U: TryFrom<Vec<u8>>>(
        data: Vec<String>,
    ) -> Result<Vec<U>, Error<T>> {
        let mut converted: Vec<U> = Vec::new();
        for record in data {
            let decoded = hex::decode(&record).map_err(|_| <Error<T>>::GenesisConfigError)?;
            let converted_validator: U = decoded
                .try_into()
                .map_err(|_| <Error<T>>::GenesisConfigError)?;
            converted.push(converted_validator);
        }
        Ok(converted)
    }

    // XXX expose price(String) fn

    /// Initializes a set of assets from a config value.
    ///
    /// * AssetMetadata // XXX set directly to liquidity factors, supply caps, etc.
    /// * AssetSymbols
    /// * RateModels
    /// * Symbols
    ///
    fn initialize_assets(assets: Vec<ConfigAsset>) {
        for asset in assets {
            let ticker = asset.symbol.ticker();
            if let Some(symbol) = Symbols::get(&ticker) {
                assert!(symbol == asset.symbol, "Different symbols for same ticker");
            } else {
                Symbols::insert(ticker, asset.symbol);
            }

            // TODO: specify rate model in ConfigAsset
            RateModels::insert(&asset.asset, InterestRateModel::default());
            AssetSymbols::insert(&asset.asset, asset.symbol);
        }
    }

    /// Set the initial set of validators from the genesis config
    /// Set next validators, bc they will become "current validators" upon session 0 start
    fn initialize_validators(ids: Vec<[u8; 32]>, keys: Vec<(String,)>) {
        assert!(
            !ids.is_empty(),
            "Validators must be set in the genesis config"
        );

        assert!(
            <NextValidators>::iter().count() == 0 || <Validators>::iter().count() == 0,
            "Validators are already set but they should only be set in the genesis config."
        );
        for (acc_id_bytes, key_tuple) in ids.iter().zip(keys.iter()) {
            log!("Adding Validator {}", hex::encode(acc_id_bytes));
            let acc_id: AccountId32 = AccountId32::new(*acc_id_bytes);

            let eth_address: [u8; 20] = hex::decode(&key_tuple.0)
                .unwrap_or_else(|_| panic!("Ecsda key could not be decoded"))
                .try_into()
                .expect("Ecsda public key not 20 bytes");
            assert!(
                <Validators>::get(&acc_id) == None,
                "Duplicate validator keys in genesis config"
            );
            <Validators>::insert(acc_id, ChainKeys { eth_address });
        }
    }

    /// Set the initial set of open price feed price reporters from the genesis config
    fn initialize_reporters(reporters: ConfigSetString) {
        assert!(
            !reporters.is_empty(),
            "Open price feed price reporters must be set in the genesis config"
        );
        assert!(
            PriceReporters::get().is_empty(),
            "Price reporters are already set but they should only be set in the genesis config."
        );
        let converted: Vec<[u8; 20]> = Self::hex_string_vec_to_binary_vec(reporters)
            .expect("Could not deserialize validators from genesis config");
        PriceReporters::put(converted);
    }

    /// Procedure for offchain worker to processes messages coming out of the open price feed
    pub fn process_open_price_feed(block_number: T::BlockNumber) -> Result<(), Error<T>> {
        let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_OPEN_PRICE_FEED);
        if lock.try_lock().is_err() {
            // working in another thread, no big deal
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
        // poll
        let api_response = cash_err!(
            crate::oracle::open_price_feed_request_okex(),
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
        // Create a reference to Local Storage value.
        // Since the local storage is common for all offchain workers, it's a good practice
        // to prepend our entry with the pallet name.
        let s_info = StorageValueRef::persistent(OCW_LATEST_CACHED_ETHEREUM_BLOCK);

        // Local storage is persisted and shared between runs of the offchain workers,
        // offchain workers may run concurrently. We can use the `mutate` function to
        // write a storage entry in an atomic fashion.
        //
        // With a similar API as `StorageValue` with the variables `get`, `set`, `mutate`.
        // We will likely want to use `mutate` to access
        // the storage comprehensively.
        //
        // Ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/storage/struct.StorageValueRef.html

        // XXXX TODO Add second check for:
        // Should be either the block of the latest Pending event which we haven't signed,
        // or if there are no such events, otherwise the block after the latest event, otherwise the earliest (configuration/genesis) block
        let from_block: String;
        if let Some(Some(cached_block_num)) = s_info.get::<String>() {
            // Ethereum block number has been cached, fetch events starting from the next after cached block
            log!("Last cached block number: {:?}", cached_block_num);
            from_block = events::get_next_block_hex(cached_block_num)
                .map_err(|_| <Error<T>>::EventsBlockNumberError)?;
        } else {
            // Validator's cache is empty, fetch events from the earliest block with pending events
            debug::native::info!("Block number has not been cached yet");
            let block_numbers: Vec<u32> = EthEventQueue::iter()
                .filter_map(|((block_number, log_index), status)| {
                    if match status {
                        EventStatus::<Ethereum>::Pending { signers } => true,
                        _ => false,
                    } {
                        Some(block_number)
                    } else {
                        None
                    }
                })
                .collect();
            let pending_events_block = block_numbers.iter().min();
            if (pending_events_block.is_some()) {
                let events_block: u32 = *pending_events_block.unwrap();
                from_block = events::get_next_block_hex(events_block.to_string())
                    .map_err(|_| <Error<T>>::EventsBlockNumberError)?;
            } else {
                from_block = String::from("earliest");
            }
        }
        debug::native::info!("Fetching events starting from block {:?}", from_block);

        // Since off-chain storage can be accessed by off-chain workers from multiple runs, it is important to lock
        //   it before doing heavy computations or write operations.
        // ref: https://substrate.dev/rustdocs/v2.0.0-rc3/sp_runtime/offchain/storage_lock/index.html
        //
        // There are four ways of defining a lock:
        //   1) `new` - lock with default time and block expiration
        //   2) `with_deadline` - lock with default block but custom time expiration
        //   3) `with_block_deadline` - lock with default time but custom block expiration
        //   4) `with_block_and_time_deadline` - lock with custom time and block expiration
        // Here we choose the default one for now.
        let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_ETHEREUM_EVENTS);

        // We try to acquire the lock here. If failed, we know the `fetch_n_parse` part inside is being
        //   executed by previous run of ocw, so the function just returns.
        // ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/storage_lock/struct.StorageLock.html#method.try_lock
        if let Ok(_guard) = lock.try_lock() {
            match events::fetch_events(from_block) {
                Ok(starport_info) => {
                    log!("Result: {:?}", starport_info);

                    // Send extrinsics for all events
                    let _ = Self::process_lock_events(starport_info.lock_events)?;

                    // Save latest block in ocw storage
                    s_info.set(&starport_info.latest_eth_block);
                }
                Err(err) => {
                    log!("Error while fetching events: {:?}", err);
                    return Err(Error::<T>::HttpFetchingError);
                }
            }
        }
        Ok(())
    }

    fn process_lock_events(
        events: Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>,
    ) -> Result<(), Error<T>> {
        for event in events.iter() {
            log!("Processing `Lock` event and sending extrinsic: {:?}", event);

            // XXX
            let payload =
                events::to_lock_event_payload(&event).map_err(|_| <Error<T>>::HttpFetchingError)?;
            let signature =  // XXX why are we signing with eth?
                cash_err!(<Ethereum as Chain>::sign_message(&payload), <Error<T>>::InvalidSignature)?; // XXX
            let call = Call::process_eth_event(payload, signature);

            // XXX Unsigned tx for now
            let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
            if res.is_err() {
                log!("Error while sending `Lock` event extrinsic");
                return Err(Error::<T>::OffchainUnsignedLockTxError);
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
            Call::process_eth_event(payload, _signature) => {
                ValidTransaction::with_tag_prefix("CashPallet")
                    .priority(100)
                    // The transaction is only valid for next 10 blocks. After that it's
                    // going to be revalidated by the pool.
                    .longevity(10)
                    .and_provides(payload)
                    // It's fine to propagate that transaction to other peers, which means it can be
                    // created even by nodes that don't produce blocks.
                    // Note that sometimes it's better to keep it for yourself (if you are the block
                    // producer), since for instance in some schemes others may copy your solution and
                    // claim a reward.
                    .propagate(true)
                    .build()
            }
            Call::exec_trx_request(request, _signature) => {
                ValidTransaction::with_tag_prefix("CashPallet")
                    .longevity(10)
                    .and_provides(request)
                    .propagate(true)
                    .build()
            }
            Call::post_price(_, sig) => ValidTransaction::with_tag_prefix("CashPallet")
                .longevity(10)
                .and_provides(sig)
                .propagate(true)
                .build(),
            Call::publish_signature(_, sig) => ValidTransaction::with_tag_prefix("CashPallet")
                .longevity(10)
                .and_provides(sig)
                .propagate(true)
                .build(),
            _ => InvalidTransaction::Call.into(),
        }
    }
}
