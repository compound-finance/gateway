#![allow(incomplete_features)]
#![feature(array_methods)]
#![feature(associated_type_defaults)]
#![feature(const_fn_floating_point_arithmetic)]

#[macro_use]
extern crate alloc;
extern crate ethereum_client;
extern crate trx_request;

use codec::{alloc::string::String, Decode};
use frame_support::{debug, decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{
    ensure_none, ensure_root,
    offchain::{CreateSignedTransaction, SubmitTransaction},
};
use our_std::{
    convert::{TryFrom, TryInto},
    str,
    vec::Vec,
};
use sp_runtime::{
    offchain::{
        storage::StorageValueRef,
        storage_lock::{StorageLock, Time},
    },
    transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
    },
    SaturatedConversion,
};

use crate::chains::{Chain, ChainId, Ethereum};
use crate::notices::{CashExtractionNotice, EncodeNotice, Notice, NoticeId};
use crate::rates::{InterestRateModel, APR};
use crate::symbol::Symbol;
use crate::types::{
    AssetAmount, AssetBalance, AssetIndex, AssetPrice, CashIndex, CashPrincipal, ChainAccount,
    ChainAsset, ChainSignature, ChainSignatureList, ConfigSet, ConfigSetString, EncodedNotice,
    EventStatus, Fraction, MaxableAssetAmount, Nonce, NoticeStatus, Reason, ReporterSet,
    SignedPayload, Timestamp, ValidatorSet, ValidatorSig,
};

use sp_runtime::print;

mod chains;
mod core;
mod events;
mod notices;
mod oracle;
mod params;
mod rates;
mod symbol;
mod testdata;
mod trx_req;
mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

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
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// The overarching dispatch call type.
    type Call: From<Call<Self>>;
}

decl_storage! {
    trait Store for Module<T: Config> as Cash {
        /// The timestamp of the previous block (or defaults to timestamp of the genesis block).
        LastBlockTimestamp get(fn last_block_timestamp) config(): Timestamp;

        // XXX we also need mapping of public (identity, babe) key to other keys
        /// The current set of allowed validators, and their associated keys.
        Validators get(fn validators): ValidatorSet; // XXX

        /// The upcoming set of allowed validators, and their associated keys (or none).
        NextValidators get(fn next_validators): Option<ValidatorSet>; // XXX

        /// An index to track interest earned by CASH holders and owed by CASH borrowers.
        GlobalCashIndex get(fn cash_index): CashIndex;

        /// The upcoming base rate change for CASH and when, if any.
        CashYieldNext get(fn cash_yield_next): Option<(APR, Timestamp)>;

        /// The current APR on CASH held, and the base rate paid by borrowers.
        CashYield get(fn cash_yield): APR;

        /// The fraction of borrower interest that is paid to the protocol.
        Spreads get(fn spread): map hasher(blake2_128_concat) ChainAsset => Fraction;

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

        RateModels get(fn model): map hasher(blake2_128_concat) ChainAsset => InterestRateModel;

        /// The last used nonce for each account, initialized at zero.
        Nonces get(fn nonces): map hasher(blake2_128_concat) ChainAccount => Nonce;

        /// The mapping of (status of) events witnessed on Ethereum, by event id.
        EthEventQueue get(fn eth_event_queue): map hasher(blake2_128_concat) chains::eth::EventId => Option<EventStatus<Ethereum>>;

        /// The mapping of (status of) notices to be signed for Ethereum, by notice id.
        EthNoticeQueue get(fn eth_notice_queue): map hasher(blake2_128_concat) NoticeId => Option<NoticeStatus>;

        // XXX
        // AssetInfo[asset];
        // LiquidationIncentive;

        /// Mapping of assets to symbols.
        AssetSymbols get(fn asset_symbol): map hasher(blake2_128_concat) ChainAsset => Option<Symbol>;

        /// Mapping of strings to symbols.
        Symbols get(fn symbol): map hasher(blake2_128_concat) String => Option<Symbol>;

        /// Mapping of latest prices for each asset symbol.
        Prices get(fn price): map hasher(blake2_128_concat) Symbol => AssetPrice;

        /// Mapping of assets to the last time their price was updated.
        PriceTimes get(fn price_time): map hasher(blake2_128_concat) Symbol => Timestamp;

        /// Ethereum addresses of open oracle price reporters.
        PriceReporters get(fn reporters): ReporterSet; // XXX if > 1, how are we combining?

        // XXX delete me (part of magic extract)
        pub CashBalance get(fn cash_balance): map hasher(blake2_128_concat) ChainAccount => Option<AssetAmount>;
    }
    add_extra_genesis {
        config(validators): ConfigSetString;
        config(reporters): ConfigSetString;
        config(symbols): ConfigSet<(String, u8)>; // XXX initialize symbol from string: (ticker, decimals)
        config(asset_symbol): ConfigSetString;
        build(|config| {
            Module::<T>::initialize_validators(config.validators.clone());
            Module::<T>::initialize_reporters(config.reporters.clone());
            Module::<T>::initialize_symbols(config.symbols.clone());
            Module::<T>::initialize_asset_maps(config.asset_symbol.clone());
        })
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId, //XXX seems to require a where clause with reference to T to compile
    {
        XXXPhantomFakeEvent(AccountId), // XXX

        /// XXX -- For testing
        GoldieLocks(ChainAsset, ChainAccount, AssetAmount),

        /// XXX
        MagicExtract(AssetAmount, ChainAccount, Notice),

        /// An Ethereum event was successfully processed. [payload]
        ProcessedEthEvent(SignedPayload),

        /// An Ethereum event failed during processing. [payload, reason]
        FailedProcessingEthEvent(SignedPayload, Reason),

        /// Signed notice. [chain_id, notice_id, message, signatures]
        SignedNotice(ChainId, NoticeId, EncodedNotice, ChainSignatureList),
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

        // XXX delete with magic extract
        /// Failure in internal logic
        Failed,
    }
}

macro_rules! r#cash_err {
    ($expr:expr, $new_err:expr) => {
        match $expr {
            core::result::Result::Ok(val) => Ok(val),
            core::result::Result::Err(err) => {
                print(format!("Error {:#?}", err).as_str());
                debug::native::error!("Error {:#?}", err);
                Err($new_err)
            }
        }
    };
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
            print("submit_trx_request");
            let request_str: &str = str::from_utf8(&request[..]).map_err(|_| <Error<T>>::TrxRequestParseError)?;
            print(request_str);
            // // TODO: Add more error information here
            let sender = cash_err!(signature.recover(&request), <Error<T>>::InvalidSignature)?; // XXX error from reason?
            let trx_request = cash_err!(trx_request::parse_request(request_str), <Error<T>>::TrxRequestParseError)?;

            match trx_request {
                trx_request::TrxRequest::MagicExtract(amount, account) => {
                    cash_err!(magic_extract_internal::<T>(sender, account.into(), amount.into()), Error::<T>::Failed)?; // TODO: How to handle wrapped errors?
                    Ok(())
                }
            }
        }

        #[weight = 1] // XXX how are we doing weights?
        pub fn process_eth_event(origin, payload: SignedPayload, signature: ValidatorSig) -> dispatch::DispatchResult { // XXX sig
            print(format!("process_eth_event(origin,payload,sig): {} {}", hex::encode(&payload), hex::encode(&signature)).as_str());
            ensure_none(origin)?;

            // XXX do we want to store/check hash to allow replaying?
            // TODO: use more generic function?
            let signer: crate::types::ValidatorKey = cash_err!(
                compound_crypto::eth_recover(&payload[..], &signature, false),  // XXX why is this using eth for validator sig though?
                Error::<T>::InvalidSignature)?;

            print(format!("signer: {}", hex::encode(&signer[..])).as_str());
            // XXX
            // XXX require(signer == known validator); // XXX
            // Check that signer is a known validator, otherwise throw an error
            let validators = <Validators>::get();
            if !validators.contains(&signer) {
                print(format!("Signer of a payload is not a known validator {:?}, validators are {:?}", signer, validators).as_str());
                return Err(Error::<T>::UnknownValidator)?
            }

            let event = chains::eth::Event::decode(&mut payload.as_slice()).map_err(|_| <Error<T>>::DecodeEthereumEventError)?; // XXX
            let status = <EthEventQueue>::get(event.id).unwrap_or(EventStatus::<Ethereum>::Pending { signers: vec![] }); // XXX
            match status {
                EventStatus::<Ethereum>::Pending { signers } => {
                    // XXX sets?
                    if signers.contains(&signer) {
                        print(format!("Validator has already signed this payload {:?}", signer).as_str());
                        return Err(Error::<T>::AlreadySigned)?
                    }

                    // Add new validator to the signers
                    let mut signers_new = signers.clone();
                    signers_new.push(signer.clone()); // XXX unique add to set?

                    // XXX
                    // let signers_new = {signer | signers};
                    // if len(signers_new & Validators) > 2/3 * len(Validators) {
                    if signers_new.len() > validators.len() * 2 / 3 {
                        match core::apply_eth_event_internal::<T>(event) {
                            Ok(()) => {
                                EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Done);
                                Self::deposit_event(RawEvent::ProcessedEthEvent(payload));
                                Ok(())
                            }

                            Err(reason) => {
                                EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Failed { hash: Ethereum::hash_bytes(payload.as_slice()), reason: reason });
                                Self::deposit_event(RawEvent::FailedProcessingEthEvent(payload, reason));
                                Ok(())
                            }
                        }
                    } else {
                        EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Pending { signers: signers_new });
                        Ok(())
                    }
                }
                // XXX potential retry logic to allow retrying Failures
                EventStatus::<Ethereum>::Failed { hash, .. } => {
                    // XXX require(compute_hash(payload) == hash, "event data differs from failure");
                    match core::apply_eth_event_internal::<T>(event) {
                        Ok(_) => {
                            EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Done);
                            Self::deposit_event(RawEvent::ProcessedEthEvent(payload));
                            Ok(())
                        }

                        Err(new_reason) => {
                            EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Failed { hash, reason: new_reason});
                            Self::deposit_event(RawEvent::FailedProcessingEthEvent(payload, new_reason));
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
        pub fn publish_eth_signature(origin, notice_id: NoticeId, signature: ValidatorSig) -> dispatch::DispatchResult {
            //let signer = recover(payload); // XXX
            //require(signer == known validator); // XXX

            let status = <EthNoticeQueue>::get(notice_id).unwrap_or(NoticeStatus::Missing);
            match status {
                NoticeStatus::Missing => {
                    Ok(())
                }

                NoticeStatus::Pending { signers, signatures, notice } => {
                    // XXX sets?
                    // let signers_new = {signer | signers};
                    // let signatures_new = {signature | signatures };
                    // if len(signers_new & Validators) > 2/3 * len(Validators) {
                           EthNoticeQueue::insert(notice_id, NoticeStatus::Done);
                           Self::deposit_event(RawEvent::SignedNotice(ChainId::Eth, notice_id, notice.encode_ethereum_notice(), signatures)); // XXX generic
                           Ok(())
                    // } else {
                    //     EthNoticeQueue::insert(event.id, NoticeStatus::Pending { signers: signers_new, signatures, notice});
                    //     Ok(())
                    // }
                }

                NoticeStatus::Done => {
                    // TODO: Eventually we should be deleting here (based on monotonic ids and signing order)
                    Ok(())
                }
            }
        }

        /// Offchain Worker entry point.
        fn offchain_worker(block_number: T::BlockNumber) {
            debug::native::info!("Hello World from offchain workers from block {} !", block_number);

            let result = Self::fetch_events_with_lock();
            if let Err(e) = result {
                debug::native::error!("offchain_worker error during fetch events: {:?}", e);
            }
            Self::process_eth_notices(block_number);
            // todo: do this less often perhaps once a minute?
            // a really simple idea to do it once a minute on average is to just use probability
            // and only update the feed every now and then. It is a function of how many validators
            // are running.
            let result = Self::process_open_price_feed(block_number);
            if let Err(e) = result {
                debug::native::error!("offchain_worker error during open price feed processing: {:?}", e);
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

        debug::native::info!(
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

    /// Set the initial set of validators from the genesis config
    fn initialize_validators(validators: ConfigSetString) {
        assert!(
            !validators.is_empty(),
            "Validators must be set in the genesis config"
        );
        assert!(
            Validators::get().is_empty(),
            "Validators are already set but they should only be set in the genesis config."
        );
        let converted: ValidatorSet = Self::hex_string_vec_to_binary_vec(validators)
            .expect("Could not deserialize validators from genesis config");
        Validators::put(converted);
    }

    // XXX actually symbols is a map with decimals
    /// Set the initial set of supported symbols from the genesis config
    fn initialize_symbols(symbols: ConfigSet<(String, u8)>) {
        assert!(
            !symbols.is_empty(),
            "Symbols must be set in the genesis config"
        );

        for (ticker, decimals) in symbols {
            // XXX
            let mut chars: Vec<char> = ticker.chars().collect();
            chars.resize(12, 0 as char);
            let raw: [char; 12] = chars.try_into().expect("bad ticker");
            Symbols::insert(ticker, Symbol(raw, decimals));
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

    /// Initializes several maps that use asset as their key to sane default values explicitly
    ///
    /// Maps initialized by this function
    ///
    /// * RateModels
    /// * AssetSymbols
    ///
    fn initialize_asset_maps(asset_symbols: ConfigSetString) {
        assert!(
            !asset_symbols.is_empty(),
            "asset_symbols must be set in the genesis config"
        );

        for record in asset_symbols {
            let mut tokens = record.split(":");
            let ticker = tokens.next().expect("No ticker present");
            let symbol = Symbols::get(ticker).expect("Ticker not a valid symbol");
            let chain = tokens.next().expect("No blockchain present");
            let address = tokens.next().expect("No address present");
            assert!(
                tokens.next().is_none(),
                "Too many colons in price key mapping element"
            );
            assert!(chain == "ETH", "Invalid blockchain");

            let decoded = hex::decode(&address).expect("Address is not in hex format");
            let decoded_arr: <Ethereum as Chain>::Address =
                decoded.try_into().expect("Invalid Ethereum address");

            let chain_asset: ChainAsset = ChainAsset::Eth(decoded_arr);

            // XXX
            // todo: we may want to do better with genesis having configurable initial interest rate models
            RateModels::insert(&chain_asset, InterestRateModel::default());
            AssetSymbols::insert(&chain_asset, symbol);
        }
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
                print(
                    format!(
                        "Open oracle prices for timestamp {:?} has been already posted",
                        timestamp
                    )
                    .as_str(),
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

    pub fn process_eth_notices(block_number: T::BlockNumber) {
        for notice in EthNoticeQueue::iter_values() {
            // find parent
            // id = notice.gen_id(parent)

            // XXX
            // submit onchain call for aggregating the price
            // let payload = notices::to_payload(notice);
            // let call = Call::publish_eth_signature(payload);

            // Unsigned tx
            // SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
        }
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
            debug::native::info!("Last cached block number: {:?}", cached_block_num);
            from_block = events::get_next_block_hex(cached_block_num)
                .map_err(|_| <Error<T>>::HttpFetchingError)?;
        } else {
            // Validator's cache is empty, fetch events from the earliest available block
            debug::native::info!("Block number has not been cached yet");
            from_block = String::from("earliest");
        }

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
                    debug::native::info!("Result: {:?}", starport_info);

                    // Send extrinsics for all events
                    let _ = Self::process_lock_events(starport_info.lock_events)?;

                    // Save latest block in ocw storage
                    s_info.set(&starport_info.latest_eth_block);
                }
                Err(err) => {
                    debug::native::info!("Error while fetching events: {:?}", err);
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
            debug::native::info!("Processing `Lock` event and sending extrinsic: {:?}", event);

            // XXX
            let payload =
                events::to_lock_event_payload(&event).map_err(|_| <Error<T>>::HttpFetchingError)?;
            let signature =  // XXX why are we signing with eth?
                cash_err!(<Ethereum as Chain>::sign_message(&payload), <Error<T>>::InvalidSignature)?; // XXX
            let call = Call::process_eth_event(payload, signature);

            // XXX Unsigned tx for now
            let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
            if res.is_err() {
                debug::native::error!("Error while sending `Lock` event extrinsic");
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
            Call::process_eth_event(payload, signature) => {
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
            Call::exec_trx_request(request, signature) => {
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
            _ => InvalidTransaction::Call.into(),
        }
    }
}

// XXX this function is temporary and will be deleted after we reach a certain point
pub fn magic_extract_internal<T: Config>(
    _sender: ChainAccount,
    account: ChainAccount,
    max_amount: MaxableAssetAmount,
) -> Result<(), Error<T>> {
    let _ = runtime_interfaces::config_interface::get(); // XXX where are we actually using this?

    let amount = max_amount.get_max_value(&|| 5);

    // Update storage -- TODO: increment this-- sure why not?
    let curr_cash_balance: AssetAmount = <CashBalance>::get(&account).unwrap_or_default();
    let next_cash_balance: AssetAmount = curr_cash_balance + amount;
    <CashBalance>::insert(&account, next_cash_balance);

    let _now = <frame_system::Module<T>>::block_number().saturated_into::<u8>();
    // Add to Notice Queue
    // Note: we need to generalize this since it's not guaranteed to be Eth
    let notice_id = NoticeId(0, 0); // XXX need to keep state of current gen/within gen for each, also parent
                                    // TODO: This asset needs to match the chain-- which for Cash, there might be different
                                    //       versions of a given asset per chain
    let notice = Notice::CashExtractionNotice(match account {
        ChainAccount::Eth(eth_account) => CashExtractionNotice::Eth {
            id: notice_id,
            parent: [0u8; 32],
            account: eth_account,
            amount,
            cash_index: 1000,
        },
    });

    // TODO: Why is this Eth notice queue? Should this be specific to Eth?
    EthNoticeQueue::insert(
        notice_id,
        NoticeStatus::Pending {
            signers: vec![],
            signatures: ChainSignatureList::Eth(vec![]),
            notice: notice.clone(),
        },
    );

    let encoded_notice = notice.encode_ethereum_notice();
    let signature = cash_err!(
        <Ethereum as Chain>::sign_message(&encoded_notice[..]),
        Error::<T>::InvalidSignature
    )?; // XXX

    // Emit an event or two.
    <Module<T>>::deposit_event(RawEvent::MagicExtract(amount, account, notice.clone()));
    <Module<T>>::deposit_event(RawEvent::SignedNotice(
        ChainId::Eth,
        notice_id,
        encoded_notice,
        ChainSignatureList::Eth(vec![signature]),
    )); // XXX signatures

    // Return a successful DispatchResult
    Ok(())
}
